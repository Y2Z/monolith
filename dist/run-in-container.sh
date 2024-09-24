#!/bin/sh

DOCKER=docker
if which podman 2>&1 > /dev/null; then
    DOCKER=podman
fi
ORG_NAME=y2z
PROG_NAME=monolith

$DOCKER run --rm $ORG_NAME/$PROG_NAME "$@"
