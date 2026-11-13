//! Safari's .webarchive files: property lists holding the document, and what it's made of.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use url::Url;

use crate::mhtml::{ArchivedAsset, MhtmlArchive};

const SIGNATURE: &[u8] = b"bplist00";
const TRAILER_SIZE: usize = 32;
const MAX_DEPTH: usize = 32;

/// Objects can be referred to from many places, each of them gets read once, and shared.
#[derive(Clone)]
enum Plist {
    Array(Rc<[Plist]>),
    Data(Rc<[u8]>),
    Dictionary(Rc<HashMap<String, Plist>>),
    Other,
    String(Rc<str>),
}

impl Plist {
    fn get(&self, key: &str) -> Option<&Plist> {
        match self {
            Plist::Dictionary(dictionary) => dictionary.get(key),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<String> {
        match self {
            Plist::String(string) => Some(string.to_string()),
            _ => None,
        }
    }
}

struct Reader<'a> {
    /// Objects that have been read already
    objects: RefCell<HashMap<usize, Plist>>,
    /// Objects that are being read, coming across one of them again means going in circles
    reading: RefCell<HashSet<usize>>,
    data: &'a [u8],
    offsets: &'a [u8],
    offset_size: usize,
    reference_size: usize,
}

fn read_number(bytes: &[u8]) -> Option<usize> {
    if bytes.len() > 8 {
        return None;
    }

    Some(
        bytes
            .iter()
            .fold(0_u64, |number, byte| (number << 8) | *byte as u64) as usize,
    )
}

impl Reader<'_> {
    fn object_offset(&self, object: usize) -> Option<usize> {
        let start = object.checked_mul(self.offset_size)?;

        read_number(
            self.offsets
                .get(start..start.checked_add(self.offset_size)?)?,
        )
    }

    /// Long objects have their length stored in a number of its own, which comes right after.
    fn read_length(&self, marker: u8, position: usize) -> Option<(usize, usize)> {
        let length = (marker & 0x0f) as usize;
        if length != 0x0f {
            return Some((length, position + 1));
        }

        let size_marker = *self.data.get(position + 1)?;
        let size = 1_usize.checked_shl((size_marker & 0x0f) as u32)?;
        let start = position + 2;

        Some((
            read_number(self.data.get(start..start.checked_add(size)?)?)?,
            start + size,
        ))
    }

    fn read_references(&self, start: usize, count: usize) -> Option<Vec<usize>> {
        let size = count.checked_mul(self.reference_size)?;

        self.data
            .get(start..start.checked_add(size)?)?
            .chunks(self.reference_size)
            .map(read_number)
            .collect()
    }

    fn read_object(&self, object: usize, depth: usize) -> Option<Plist> {
        if let Some(plist) = self.objects.borrow().get(&object) {
            return Some(plist.clone());
        }
        if depth > MAX_DEPTH || !self.reading.borrow_mut().insert(object) {
            return None;
        }

        let plist = self.parse_object(object, depth);
        self.reading.borrow_mut().remove(&object);
        if let Some(plist) = &plist {
            self.objects.borrow_mut().insert(object, plist.clone());
        }

        plist
    }

    fn parse_object(&self, object: usize, depth: usize) -> Option<Plist> {
        let position = self.object_offset(object)?;
        let marker = *self.data.get(position)?;

        Some(match marker >> 4 {
            0x4 => {
                let (length, start) = self.read_length(marker, position)?;
                Plist::Data(self.data.get(start..start.checked_add(length)?)?.into())
            }
            0x5 => {
                let (length, start) = self.read_length(marker, position)?;
                let bytes = self.data.get(start..start.checked_add(length)?)?;
                Plist::String(String::from_utf8_lossy(bytes).into())
            }
            0x6 => {
                let (length, start) = self.read_length(marker, position)?;
                let bytes = self
                    .data
                    .get(start..start.checked_add(length.checked_mul(2)?)?)?;
                let units: Vec<u16> = bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|unit| u16::from_be_bytes([unit[0], unit[1]]))
                    .collect();
                Plist::String(String::from_utf16_lossy(&units).into())
            }
            0xa => {
                let (length, start) = self.read_length(marker, position)?;
                Plist::Array(
                    self.read_references(start, length)?
                        .into_iter()
                        .map(|item| self.read_object(item, depth + 1))
                        .collect::<Option<Vec<_>>>()?
                        .into(),
                )
            }
            0xd => {
                let (length, start) = self.read_length(marker, position)?;
                let keys = self.read_references(start, length)?;
                let values = self.read_references(start + length * self.reference_size, length)?;
                let mut dictionary = HashMap::new();
                for (key, value) in keys.into_iter().zip(values) {
                    if let Plist::String(key) = self.read_object(key, depth + 1)? {
                        dictionary.insert(key.to_string(), self.read_object(value, depth + 1)?);
                    }
                }
                Plist::Dictionary(Rc::new(dictionary))
            }
            _ => Plist::Other,
        })
    }
}

fn parse_plist(data: &[u8]) -> Option<Plist> {
    if !data.starts_with(SIGNATURE) || data.len() < SIGNATURE.len() + TRAILER_SIZE {
        return None;
    }

    let trailer = &data[data.len() - TRAILER_SIZE..];
    let offset_size = trailer[6] as usize;
    let reference_size = trailer[7] as usize;
    let objects = read_number(&trailer[8..16])?;
    let root = read_number(&trailer[16..24])?;
    let offsets_start = read_number(&trailer[24..32])?;
    if !(1..=8).contains(&offset_size) || !(1..=8).contains(&reference_size) {
        return None;
    }

    let reader = Reader {
        objects: RefCell::new(HashMap::new()),
        reading: RefCell::new(HashSet::new()),
        data,
        offsets: data
            .get(offsets_start..offsets_start.checked_add(objects.checked_mul(offset_size)?)?)?,
        offset_size,
        reference_size,
    };

    reader.read_object(root, 0)
}

fn parse_resource(resource: &Plist) -> Option<(Vec<Url>, ArchivedAsset)> {
    let Plist::Data(data) = resource.get("WebResourceData")? else {
        return None;
    };
    let url = Url::parse(&resource.get("WebResourceURL")?.as_string()?).ok()?;

    Some((
        vec![url.clone()],
        ArchivedAsset {
            data: data.to_vec(),
            url: Some(url),
            media_type: resource
                .get("WebResourceMIMEType")
                .and_then(Plist::as_string)
                .unwrap_or_default(),
            charset: resource
                .get("WebResourceTextEncodingName")
                .and_then(Plist::as_string)
                .unwrap_or_default(),
            response_headers: None,
        },
    ))
}

/// Frames are archives of their own, they end up being resources like any other.
fn collect_resources(archive: &Plist, assets: &mut Vec<(Vec<Url>, ArchivedAsset)>) {
    assets.extend(archive.get("WebMainResource").and_then(parse_resource));

    if let Some(Plist::Array(resources)) = archive.get("WebSubresources") {
        assets.extend(resources.iter().filter_map(parse_resource));
    }
    if let Some(Plist::Array(frames)) = archive.get("WebSubframeArchives") {
        for frame in frames.iter() {
            collect_resources(frame, assets);
        }
    }
}

pub fn is_webarchive(data: &[u8]) -> bool {
    parse_plist(data).is_some_and(|plist| plist.get("WebMainResource").is_some())
}

pub fn parse_webarchive(data: &[u8]) -> Option<MhtmlArchive> {
    let plist = parse_plist(data)?;
    plist.get("WebMainResource").and_then(parse_resource)?;

    let mut assets = vec![];
    collect_resources(&plist, &mut assets);

    Some(MhtmlArchive::from_assets(assets))
}
