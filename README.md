# grunt
Minimal RSS reader backend built with Rust.

*note: this project is in very early stages of development*

## CLI usage
```
Arguments:
  -h, --help               print help message
  -p, --port PORT          port to bind by the service (default: 4000)
  -d, --db-path DB-PATH    directory to store the database in (default: db)
  -u, --user USER          basic auth password
  -P, --password PASSWORD  basic auth user name
  -i, --interval-minutes INTERVAL-MINUTES
                           refresh time interval in minutes (default: 30)
```
## REST usage
- `POST /admin/subscriptions`
  - adds a new subscription
  - expected body:
    ```json
    { "feed_url": "https://feeds.bbci.co.uk/news/world/rss.xml" }
    ```
- `POST /admin/jobs/refresh`
  - triggers a refresh of all feeds

*all endpoints require BasicAuth credentials*

## features
- serves a [Feedbin API](https://github.com/feedbin/feedbin-api), supported by clients like [Fluent Reader](https://github.com/yang991178/fluent-reader)
- no external dependencies, installation is as simple as copying the executable
- parallelized feed sync
- very low resource use (uses <20MB of RAM), can be run on Pi and similar
- zero-copy parsing (for both RSS feeds and database entries)

## client integration
You can connect to a local instance of grunt with FluentReader using configuration below

![image](https://user-images.githubusercontent.com/11986158/166170369-b7bc881d-6b7b-47b9-bc50-9968e5c46ef5.png)
