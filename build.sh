#!/bin/bash
docker run --rm -ti -v `pwd`:/proj -v $HOME/.cargo:/root/.cargo -w /proj pi-builder bash -c '$HOME/.cargo/bin/cargo build --release --target=arm-unknown-linux-musleabihf'
