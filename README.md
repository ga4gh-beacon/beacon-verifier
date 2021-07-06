# Beacon verifier

![beacon-verifier.001.png](assets/diagram.jpeg)

## Installation

> Requirements: [Rust](https://www.rust-lang.org/tools/install)

```sh
cargo install beacon-verifier
```

## Usage

You can specify one or multiple urls:

```sh
beacon-verifier https://beacon-url.com/
```

> By default, the [Beacon 2 specification](https://github.com/ga4gh-beacon/beacon-v2-Models/tree/main/BEACON-V2-draft4-Model) is being used. But you can provide your own spec with the `--spec` option. The spec should follow the [Beacon Framework](https://github.com/ga4gh-beacon/beacon-framework-v2).

```sh
beacon-verifier --spec https://beacon-spec.com/ https://beacon-url.com/
```

## Output

The output is a json file that is saved in the current directory. You can override the location of the output with the `--output` option.

```sh
beacon-verifier --output /path/to/output https://beacon-url.com/
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
                "valid": true
            },
            "variants": {
                "name": "Variants",
                "url": "https://.../variants",
                "valid": false
            },
            "variants": {
                "name": "Variants",
                "url": "https://.../variants",
                "valid": null
            }
            //...
        }
    },
    //...
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
    valid: Option<bool>
}
```
