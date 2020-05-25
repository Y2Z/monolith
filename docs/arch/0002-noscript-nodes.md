# 2. NOSCRIPT nodes

Date: 2020-04-16

## Status

Accepted

## Context

HTML pages can contain `noscript` nodes, which reveal their contents only in case when JavaScript is not available. Most of the time they contain hidden messages that inform about certain JavaScript-dependent features not being operational, however sometimes can also feature media assets or even iframes.

## Decision

When the document is being saved with or without JavaScript, each `noscript` node should be preserved while its children need to be processed exactly the same way as the rest of the document. This approach will ensure that even hidden remote assets are embedded — since those hidden elements may have to be displayed later in a browser that has JavaScript turned off. An option should be available to "unwrap" all `noscript` nodes in order to make their contents always visible in the document, complimenting the "disable JS" function of the program.

## Consequences

Saved documents will have contents of all `noscript` nodes processed as if they are part of the document's DOM, therefore properly display images encapsulated within `noscript` nodes when being viewed in browsers that have JavaScript turned off (or have no JavaScript support in the first place). The new option to "unwrap" `noscript` elements will help the user ensure that the resulting document always represents what the original web page looked like in a browser that had JavaScript turned off.
