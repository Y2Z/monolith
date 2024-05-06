FROM node:alpine

RUN apk --no-cache add curl bash git monolith jq
RUN npm -g install apify-cli
COPY .actor .actor
CMD ./.actor/bin/actor.sh
