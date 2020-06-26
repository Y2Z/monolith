# 4. Reload and location `meta` nodes

Date: 2020-06-25

## Status

Accepted

## Context

HTML documents may contain `meta` tags capable of automatically refreshing the page or redirecting to another location.

## Decision

Since the resulting document is saved to disk and generally not intended to be served over the network, it only makes sense to remove `meta` nodes that have `http-equiv` attribute equal to "Refresh" or "Location" in order to prevent them from forcing page to reload or redirect to another location.

## Consequences

Monolith will ensure that saved documents do not contain `meta` tags capable of changing location or reloading page.
