# consulbeat

## Setup

```bash
mkdir /tmp/consul
chmod o+w /tmp/consul

# start consul and script to cause errors
# you should now have a new consul log file with errors every 30 seconds
docker compose up consul

# start consulbeat
export SENTRY_DSN="<your dsn here>"
docker compose up --build consulbeat
```

## Possible Future Improvements

- Call v1/agent/self to get metadata about the local consul instance
- Send parsed logs into ELK or use Open Telemetry