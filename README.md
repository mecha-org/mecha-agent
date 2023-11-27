# mecha-agent

Mecha Agent is the service running on device to manage the connection with Mecha Services includes features for provisioning, messaging and telemetry. Check the [docs](https://docs.mecha.so) for more information.

## Dependencies

Rust: `1.70 or above`
## Running the mecha-agent on local

1. Clone this repository

```sh
$ git clone https://github.com/mecha-org/mecha-agent
```

2. Create your settings.yml using the `settings.yml.example` provided, update the defaults as necessary

```sh
$ cd mecha-agent
$ cp settings.yml.example settings.yml
```

3. Run the `mecha-agent` using cargo run

```sh
$ cargo run -- -s ./settings.yml
```

4. To generate the release build

```sh
$ cargo build --release
$ ./target/release/mecha_agent_server -s ./settings.yml
```

## Running via Docker

1. Ensure you have `settings.yml` in the repository root directory generated with the required settings for your docker.

2. Build the docker image using the `Dockerfile` provided in the root directory

```sh
$ docker build -t mecha-org/mecha-agent .
```

3. Run the docker image, with the port exposed in your settings.yml for the grpc server

```sh
$ docker run -p 3001:3001 mecha-org/mecha-agent
```