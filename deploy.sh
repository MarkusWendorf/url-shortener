COMPOSE_FILE=compose.yml
DETACHED=true

# Make sure the host and key is setup in ~/.ssh/config
HOST=root@ec2-3-67-202-144.eu-central-1.compute.amazonaws.com
export DOCKER_HOST="ssh://$HOST"

docker compose -f $COMPOSE_FILE up --build --detach=$DETACHED --remove-orphans