# cloudflare-updater

Extremely simple DDNS for Cloudflare

## Description

Periodically monitors the container's public IP address and updates a Cloudflare DNS record to point to that address.

## Configuration

| Variable name | Description                                                    | Required | Default |
| ------------- | -------------------------------------------------------------- | -------- | ------- |
| CF_INTERVAL   | How often the public IP will be checked, in seconds.           | false    | 120     |
| CF_ZONE_ID    | The ID of the Cloudflare Zone containing the DNS record.       | true     | -       |
| CF_RECORD_ID  | The ID of the DNS record.                                      | true     | -       |
| CF_TOKEN      | A Cloudflare API token that can read and write the DNS record. | true     | -       |

## Usage

Docker:

```sh
docker run -e CF_INTERVAL=120 -e CF_ZONE_ID=some_zone_id -e CF_RECORD_ID=some_record_id -e CF_TOKEN=some_token -d benjuan26/cloudflare-updater:latest
```

Docker-compose:

```yaml
---
version: '3.8'
services:
  cloudflare-updater:
    image: benjuan26/cloudflare-updater:latest
    container_name: cloudflare-updater
    environment:
      CF_INTERVAL: 120 # optional
      CF_ZONE_ID: some_zone_id
      CF_RECORD_ID: some_record_id
      CF_TOKEN: some_token
    restart: unless-stopped
```
