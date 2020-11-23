# 8. Base Tag

Date: 2020-11-22

## Status

Accepted

## Context

HTML documents may contain `base` tag within `head`, which influences URL resolution prefix for anchor and relative links as well as dynamically loaded resources. Sometimes to make certain saved pages function closer to how they originally operated, the `base` tag specifying the source page's URL may need to be added to the document.

## Decision

Adding the `base` tag should be optional. Saved documents should not contain the `base` tag unless it was requested by the user, or unless the document originally had the `base` tag in it. Only documents donwloaded from remote resources should be able to obtain a new `base` tag, existing `base` tags within documents saved from data URLs and local resources should be kept intact.
The existing `href` attribute's value of the original `base` tag should be used for resolving document's relative links instead of document's own URL.
There can be only one such tag. If multiple `base` tags are provided, only the first encountered tag will end up being used.

## Consequences

In case the remote document had the `base` tag in it:
 - By default: the `href` attribute should be resolved to a full URL if it's relative, kept empty in case it was empty or non-existent, all other attributes of that tag should be kept intact.
 - If `base` tag was requested to be added: the exsting `base` tag's `href` attribute should be set to page's full URL, all other attributes should be kept intact.

In case the remote document didn't have the `base` tag in it:
 - By default: no `base` tag is added to the document, it gets saved to disk without having one.
 - If `base` tag was requested to be added: the added `base` tag should contain only one attribute `href`, equal to the remote URL of that HTML document.
