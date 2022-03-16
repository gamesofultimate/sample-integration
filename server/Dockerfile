FROM debian:stable-slim

WORKDIR /opt/app

RUN apt-get update -y && apt-get install -y ca-certificates
RUN dpkg-reconfigure -p critical ca-certificates

COPY ./runner/dist ./dist
COPY ./.env.production ./.env

CMD ["/opt/app/dist/runner"]
