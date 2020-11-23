# 3. Network request timeout

Date: 2020-02-15

## Status

Accepted

## Context

A slow network connection and overloaded server may negatively impact network response time.

## Decision

Make the program simulate behavior of popular web browsers and CLI tools, where the default network response timeout is most often set to 120 seconds.

Instead of featuring retries for timed out network requests, the program should have an option to adjust the timeout length, along with making it indefinite when given "0" as its value.

## Consequences

The user is able to retrieve resources that have long response time, as well as obtain full control over how soon, and if at all, network requests should time out.
