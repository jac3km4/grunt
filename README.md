# grunt
Minimal RSS reader built with Rust.

*note: this project is alpha quality*

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
- `POST /admin/jobs/refresh`
  - triggers a refresh of all feeds
- [standard Feedbin endpoints](https://github.com/feedbin/feedbin-api)

*all endpoints require BasicAuth credentials*

## features
- serves a [Feedbin API](https://github.com/feedbin/feedbin-api), supported by clients like [Fluent Reader](https://github.com/yang991178/fluent-reader)
- no external dependencies, installation is as simple as copying the executable
- parallelized feed sync
- very low resource use (uses <20MB of RAM), can be run on Pi and similar
- zero-copy parsing of RSS feeds
- lightweight built-in frontend

## frontend
Grunt comes with a lightweight frontend (source available [here](https://github.com/jac3km4/grunt-frontend)).
It's available under `[grunt-host]/webui` (`localhost:4000/webui` when running locally).

<img style="float:left;" src="https://user-images.githubusercontent.com/11986158/167316129-3bf49293-0894-4485-b726-048cf551d76c.png" width="480"/>
<img style="float:clear;" src="https://user-images.githubusercontent.com/11986158/167316133-2ee48021-3deb-4648-b248-065d8cfded46.png" width="480"/>

## client integration
You can connect to grunt with clients that support the feedbin API.
For example, a valid FluentReader configuration can be seen below

![image](https://user-images.githubusercontent.com/11986158/166170369-b7bc881d-6b7b-47b9-bc50-9968e5c46ef5.png)
