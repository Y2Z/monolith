#[cfg(test)]
mod helpers {
    pub const ARCHIVE: &[u8] = include_bytes!("../_data_/webarchive/page.webarchive");
}

#[cfg(test)]
mod passing {
    use monolith::url::Url;
    use monolith::webarchive::{is_webarchive, parse_webarchive};

    use super::helpers::ARCHIVE;

    #[test]
    fn detects_archives() {
        assert!(is_webarchive(ARCHIVE));
    }

    #[test]
    fn main_resource_is_the_document() {
        let archive = parse_webarchive(ARCHIVE).unwrap();
        let root = archive.root();

        assert_eq!(root.media_type, "text/html");
        assert_eq!(root.charset, "UTF-8");
        assert_eq!(
            root.url.as_ref().unwrap().as_str(),
            "https://example.com/page.html"
        );
        assert!(String::from_utf8_lossy(&root.data).contains("Safari page"));
    }

    #[test]
    fn finds_subresources_and_frames() {
        let archive = parse_webarchive(ARCHIVE).unwrap();

        assert_eq!(archive.len(), 4);
        for path in ["style.css", "dot.gif", "frame.html"] {
            let url = Url::parse(&format!("https://example.com/{}", path)).unwrap();
            assert!(archive.contains(&url));
        }
    }

    #[test]
    fn resources_may_share_what_they_hold() {
        // Every one of these refers to the same piece of data
        let archive = parse_webarchive(include_bytes!("../_data_/webarchive/shared.webarchive"));

        assert!(is_webarchive(include_bytes!(
            "../_data_/webarchive/shared.webarchive"
        )));
        assert_eq!(archive.unwrap().len(), 5);
    }

    #[test]
    fn scripts_get_inlined_unless_they_have_to_be_waited_for() {
        use monolith::core::{MonolithOptions, create_monolithic_document_from_data};
        use monolith::session::Session;

        let options = MonolithOptions {
            silent: true,
            ..Default::default()
        };
        let archive = include_bytes!("../_data_/webarchive/scripts.webarchive").to_vec();
        let (result, _) = create_monolithic_document_from_data(
            Session::new(None, None, options),
            archive,
            None,
            None,
        )
        .unwrap();
        let result = String::from_utf8(result).unwrap();

        // Decoded the way its archive says it's encoded
        assert!(
            result.contains("<script>var word = 'caf\u{e9}';</script>"),
            "{}",
            result
        );
        // Inline scripts never load, nothing would ever get to know they have
        assert!(result.contains("<script src=\"data:text/javascript"));
        assert!(result.contains("onload=\"window.loaded = true\""));
        // Inline scripts can't wait
        assert!(result.contains("defer=\"\""));
        assert_eq!(result.matches("<script src=\"data:").count(), 2);
    }
}

#[cfg(test)]
mod failing {
    use monolith::webarchive::{is_webarchive, parse_webarchive};

    use super::helpers::ARCHIVE;

    #[test]
    fn other_files() {
        assert!(!is_webarchive(b"<html></html>"));
        assert!(!is_webarchive(b"bplist00"));
    }

    #[test]
    fn broken_archives_do_not_panic() {
        for length in 0..ARCHIVE.len() {
            assert!(parse_webarchive(&ARCHIVE[..length]).is_none());
        }

        for position in 0..ARCHIVE.len() {
            let mut broken = ARCHIVE.to_vec();
            broken[position] ^= 0xff;
            let _ = parse_webarchive(&broken);
        }
    }

    #[test]
    fn objects_referred_to_over_and_over_do_not_add_up() {
        // Every array refers to the next one twice: reading it all would take 2^30 steps
        let levels: usize = 30;
        let mut data = b"bplist00".to_vec();
        let mut offsets = vec![];
        for level in 0..levels {
            offsets.push(data.len());
            match level + 1 < levels {
                true => data.extend([0xa2, (level + 1) as u8, (level + 1) as u8]),
                false => data.push(0xa0),
            }
        }
        let offsets_start = data.len();
        for offset in &offsets {
            data.push(*offset as u8);
        }
        let mut trailer = vec![0; 6];
        trailer.extend([1, 1]);
        trailer.extend((levels as u64).to_be_bytes());
        trailer.extend(0_u64.to_be_bytes());
        trailer.extend((offsets_start as u64).to_be_bytes());
        data.extend(trailer);

        let started = std::time::Instant::now();
        assert!(!is_webarchive(&data));
        assert!(parse_webarchive(&data).is_none());
        assert!(started.elapsed().as_secs() < 5);
    }

    #[test]
    fn data_referred_to_over_and_over_does_not_fill_memory() {
        // Plenty of objects that are never used, one big piece of data, and an array that refers
        // to it ten thousand times: copying it each time would take a gigabyte
        let fillers: usize = 20_000;
        let references: usize = 10_000;
        let mut data = b"bplist00".to_vec();
        let mut offsets = vec![];
        offsets.push(data.len());
        data.extend([0xaf, 0x11]);
        data.extend((references as u16).to_be_bytes());
        for _ in 0..references {
            data.extend(1_u16.to_be_bytes());
        }
        offsets.push(data.len());
        data.extend([0x4f, 0x12]);
        data.extend(100_000_u32.to_be_bytes());
        data.extend(vec![0; 100_000]);
        for _ in 0..fillers {
            offsets.push(data.len());
            data.push(0x08);
        }
        let offsets_start = data.len();
        for offset in &offsets {
            data.extend((*offset as u32).to_be_bytes());
        }
        let mut trailer = vec![0; 6];
        trailer.extend([4, 2]);
        trailer.extend((offsets.len() as u64).to_be_bytes());
        trailer.extend(0_u64.to_be_bytes());
        trailer.extend((offsets_start as u64).to_be_bytes());
        data.extend(trailer);

        assert!(!is_webarchive(&data));
        assert!(parse_webarchive(&data).is_none());
    }
}
