#!/bin/sh

DOCKER=docker
if which podman 2>&1 > /dev/null; then
    DOCKER=podman
fi
ORG_NAME=y2z
PROG_NAME=monolith
DOCKER_IMAGE_TAG=$ORG_NAME/$PROG_NAME
TEMP_DIRECTORY=/tmp/monolith-$(openssl rand -hex 4)

mkdir TEMP_DIRECTORY

if [ -d "$TEMP_DIRECTORY" ]; then
    $DOCKER run --rm -v "$TEMP_DIRECTORY:/mnt" $DOCKER_IMAGE_TAG $@
    mv $TEMP_DIRECTORY/* .
    rmdir TEMP_DIRECTORY
else
    $DOCKER run --rm $ORG_NAME/$PROG_NAME "$@"
fi
