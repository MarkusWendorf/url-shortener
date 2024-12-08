HOST_NAME=http://ec2-3-75-88-39.eu-central-1.compute.amazonaws.com:3000

oha -r 0 -n 1000000 -m POST -T 'application/json' -d '{"url":"https://computerbase.de"}' $HOST_NAME/create-short-url
