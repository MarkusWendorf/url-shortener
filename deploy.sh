# rustup target add x86_64-unknown-linux-gnu
RUST_TARGET="x86_64-unknown-linux-gnu"

# cargo install cargo-zigbuild
cargo zigbuild --release --target ${RUST_TARGET}

HOST_NAME=ec2-user@ec2-3-75-88-39.eu-central-1.compute.amazonaws.com

rsync -avz --no-perms -O -e "ssh -i ./infra/keys/private.pem -o 'StrictHostKeyChecking no'" ./target/$RUST_TARGET/release/url-shortener $HOST_NAME:/var/url-shortener
