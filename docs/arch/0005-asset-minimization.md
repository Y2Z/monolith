# 5. Asset Minimization

Date: 2020-03-14

## Status

Accepted

## Context

It may look like a good idea to make monolith compress retrieved assets while saving the page for the purpose of reducing the resulting document's file size.

## Decision

Given that the main purpose of this program is to save pages in a convenient to store and share manner — it's mostly an archiving tool, aside from being able to tell monolith to exclude certain types of asests (e.g. images, CSS, JavaScript), it would be outside of scope of this program to implement code for compressing assets. Minimizing files before embedding them does not reduce the amount of data that needs to be transferred either. A separate tool can be used later to compress and minimize pages saved by monolith, if needed.

## Consequences

Monolith will not support modification of original document assets for the purpose of reducing their size, sticking to performing only minimal amount of modifications to the original web page — whatever is needed to provide security or exclude unwanted asset types.
