# Beacon verifier

![beacon-verifier.001.png](assets/diagram.jpeg)

## Installation

> Requirements: [Rust](https://www.rust-lang.org/tools/install)
> **Minimum Rust version**: `1.56`

```sh
cargo install beacon-verifier
```

## Usage

You can specify one or multiple urls:

```sh
beacon-verifier https://beacon-url.com/
```

> By default, the [Beacon v2 model](https://github.com/ga4gh-beacon/beacon-v2-Models/tree/main/BEACON-V2-Model) is being used. But you can provide your own model with the `--model` option. The model should follow the [Beacon Framework](https://github.com/ga4gh-beacon/beacon-framework-v2).

```sh
beacon-verifier --model https://beacon-model.com/ https://beacon-url.com/
```

Alternatively, you can specify a local path for the model:

```sh
beacon-verifier --model file://$PWD/tests/BEACON-V2-Model https://beacon-url.com/
```

## Output

The output is a JSON file written to stdout. You can redirect it to save it into a file.

```sh
beacon-verifier https://beacon-url.com/ > /path/to/output
```

### Output example

```json
[
    {
        "name": "Beacon Name",
        "url": "https://...",
        "entities": {
            "individuals": {
                "name": "Individuals",
                "url": "https://.../individuals",
                "valid": true,
                "error": null,
            },
            "variants": {
                "name": "Variants",
                "url": "https://.../variants",
                "valid": false,
                "error": "Bad schema"
            },
            "biosamples": {
                "name": "Biosamples",
                "url": "https://.../biosamples",
                "valid": null,
                "error": "Unresponsive endpoint"
            }
        }
    }
]
```

### Output format

The output is a `Vec<Beacon>` with the following format:

```rust
struct Beacon {
    name: String,
    url: String,
    entities: Vec<Entity>
}

struct Entity {
    name: String,
    url: String,
    valid: Option<bool>,
    error: Option<VerifierError>
}
```
