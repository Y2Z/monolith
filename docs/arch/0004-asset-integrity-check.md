# 4. Asset integrity check

Date: 2020-02-23

## Status

Accepted

## Context

In HTML5, `link` and `script` nodes have an attribute named
`integrity`, which lets the browser check if the remote file is
valid, mostly for the purpose of enhancing page security.

## Decision

In order to replicate browsers' behavior, the program should
perform integrity check the same way browsers do, excluding the
linked asset from the final result if such check fails.

The `integrity` attribute should be removed from nodes,
as it bears no benefit for resources embedded as data URLs.

## Consequences

Assets that fail to pass the check get excluded from the saved document.

Saved documents no longer contain integrity attributes on all `link` and `script` nodes.
