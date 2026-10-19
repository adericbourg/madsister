# The cargo-packager config of a release build, for the release workflow: Cargo.toml's, named after the package
# (madsister or madsister-snapshot, which install together; the app finds its sidecars, resources and data dir by its
# binary's name), with the madmom wheel and the extra sidecars given (Windows: ffmpeg).
# Usage, in app/ after `cargo build --release` and copying the binary to target/release/<package>:
#   cargo packager -c "$(uv run --no-project --python 3.11 release-config.py <package> <version> [sidecar...])" -f <formats>
# (uv: tomllib needs Python 3.11, which the runners' `python3` isn't always)
# A JSON string rather than a file: cargo-packager 0.11.8 fails on a config file without a `name`.
import json
import sys
import tomllib

package, version, *sidecars = sys.argv[1:]
with open("Cargo.toml", "rb") as f:
    config = tomllib.load(f)["package"]["metadata"]["packager"]
del config["before-packaging-command"]  # the workflow has built the binary already
config.update({
    "product-name": package,
    "identifier": f"dev.dericbourg.{package}",
    "version": version,  # a --config isn't completed from Cargo.toml
    "binaries": [{"path": package, "main": True}],
    "external-binaries": [f"binaries/{package}-uv", *sidecars],
    "out-dir": "target/release",
})
config["resources"].append({"src": "../engine/wheels", "target": "engine/wheels"})
json.dump(config, sys.stdout)
