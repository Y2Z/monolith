#!/bin/bash
#pwd
#find ./storage
apify actor:get-input > /dev/null
INPUT=`apify actor:get-input | jq -r .urls[] | xargs echo`
echo "INPUT: $INPUT"

for url in $INPUT; do
  # support for local usage
  # sanitize url to a safe *nix filename - replace nonalfanumerical characters
  # https://stackoverflow.com/questions/9847288/is-it-possible-to-use-in-a-filename
  # https://serverfault.com/questions/348482/how-to-remove-invalid-characters-from-filenames
  safe_filename=`echo $url | sed -e 's/[^A-Za-z0-9._-]/_/g'`
  echo "Monolith-ing $url to key $safe_filename"
  monolith $url | apify actor:set-value "$safe_filename" --contentType=text/html
  kvs_url="https://api.apify.com/v2/key-value-stores/${APIFY_DEFAULT_KEY_VALUE_STORE_ID}/records/${safe_filename}"
  result=$?
  echo "Pushing result item to the datastore"
  echo "{\"url\":\"${url}\",\"status\":\"${result}\", \"kvsUrl\":\"${kvs_url}\"}" | apify actor:push-data
done

exit 0
