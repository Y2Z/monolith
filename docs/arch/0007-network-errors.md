# 7. Network errors

Date: 2020-11-22

## Status

Accepted

## Context

Servers may return information with HTTP response codes other than `200`, however those responses may still contain useful data.

## Decision

Fail by default, notifying of the network error. Add option to continue retrieving assets by treating all response codes as `200`.

## Consequences

Monolith will fail to obtain resources with status other than `200`, unless told to ignore network errors.
