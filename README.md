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

## TODO

- factor & clean
- - neaten up log watch duration
- - test working in docker
- Add support for setting SENTRY_ENVIRONMENT (env var does work automatically?)
- Include DC as additional data