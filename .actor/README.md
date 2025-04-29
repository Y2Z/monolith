# Monolith Actor on Apify

[![Monolith Actor](https://apify.com/actor-badge?actor=snshn/monolith)](https://apify.com/snshn/monolith?fpr=snshn)

This Actor wraps [Monolith](https://crates.io/crates/monolith) to crawl a web page URL and bundle the entire content in a single HTML file, without installing and running the tool locally.

## What are Actors?
[Actors](https://docs.apify.com/platform/actors?fpr=snshn) are serverless microservices running on the [Apify Platform](https://apify.com/?fpr=snshn). They are based on the [Actor SDK](https://docs.apify.com/sdk/js?fpr=snshn) and can be found in the [Apify Store](https://apify.com/store?fpr=snshn). Learn more about Actors in the [Apify Whitepaper](https://whitepaper.actor?fpr=snshn).

## Usage

### Apify Console

1. Go to the Apify Actor page
2. Click "Run"
3. In the input form, fill in **URL(s)** to crawl and bundle
4. The Actor will run and :
    - save the bundled HTML files in the run's default key-value store
    - save the links to the KVS with original URL and monolith process exit status to the dataset


### Apify CLI

```bash
apify call snshn/monolith --input='{
  "urls": ["https://news.ycombinator.com/"]
}'
```

### Using Apify API

```bash
curl --request POST \
  --url "https://api.apify.com/v2/acts/snshn~monolith/run" \
  --header 'Content-Type: application/json' \
  --header 'Authorization: Bearer YOUR_API_TOKEN' \
  --data '{
  "urls": ["https://news.ycombinator.com/"],
  }
}'
```

## Input Parameters

The Actor accepts a JSON schema with the following structure:

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `urls` | array | Yes | - | List of URLs to monolith |
| `urls[]` | string | Yes | - | URL to monolith |


### Example Input

```json
{
  "urls": ["https://news.ycombinator.com/"],
}
```

## Output

The Actor provides three types of outputs:

### Dataset Record

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | A link to the Apify key-value store object where the monolithic html is available for download |
| `kvsUrl` | array | Yes | Exit status of the monolith process |
| `status`| number | No | The original start URL for the monolith process |

### Example Dataset Item (JSON)

```json
{
    "url": "https://news.ycombinator.com/",
    "kvsUrl": "https://api.apify.com/v2/key-value-stores/JRFLHRy9DOtdKGpdm/records/https___news.ycombinator.com_",
    "status": "0"
}
```

## Performance & Resources

- **Memory Requirements**:
  - Minimum: 4168 MB RAM
- **Processing Time**:
  - 30s per compex page like [bbc.co.uk](https://bbc.co.uk)


For more help, check the [Monolith Project documentation](https://github.com/Y2Z/monolith) or raise an issue in the [Actor page detail](https://apify.com/snshn/monolith?fpr=snshn) on Apify.


