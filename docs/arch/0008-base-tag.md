# 8. Base Tag

Date: 2020-12-25

## Status

Accepted

## Context

HTML documents may contain `base` tag, which influences resolution of anchor links and relative URLs as well as dynamically loaded resources.

Sometimes, in order to make certain saved documents function closer to how they operate while being served from a remote server, the `base` tag specifying the source page's URL may need to be added to the document.

There can be only one such tag. If multiple `base` tags are present, only the first encountered tag ends up being used.

## Decision

Adding the `base` tag should be optional — saved documents should not contain the `base` tag unless it was specified by the user, or the document originally had the `base` tag in it.

Existing `href` attribute's value of the original `base` tag should be used for resolving the document's relative links instead of document's own URL (precisely the way browsers do it).

## Consequences

#### If the base tag does not exist in the source document

- If the base tag does not exist in the source document
  - With base URL option provided
    - use the specified base URL value to retrieve assets, keep original base URL value in the document
  - Without base URL option provided
    - download document as usual, do not add base tag
-  If the base tag already exists in the source document
   - With base URL option provided
     - we overwrite the original base URL before retrieving assets, keep new base URL value in the document
   - Without base URL option provided:
     - use the base URL from the original document to retrieve assets, keep original base URL value in the document

The program will obtain ability to retrieve remote assets for non-remote sources (such as data URLs and local files).

The program will obatin ability to get rid of existing base tag values (by provind an empty one).
