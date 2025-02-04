FROM debian:stable

# Install common dependencies
RUN apt-get update && apt-get install -y libdw1 libssl-dev openssl curl && rm -rf /var/lib/apt/lists/*

# Copy both the aptos-node and aptos-faucet-service into the container
COPY ./aptos-node /usr/local/bin/aptos-node
COPY ./aptos-faucet-service /usr/local/bin/aptos-faucet-service
COPY ./genesis.blob /opt/aptos/genesis/genesis.blob
COPY ./waypoint.txt /opt/aptos/genesis/waypoint.txt
COPY ./libvoidstar.so /usr/lib/libvoidstar.so
# Copy the entrypoint script
COPY ./entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Set some defaults (overridden in docker-compose if needed)
ENV ROLE="validator"
ENV CONFIG_PATH="/opt/aptos/etc/validator.yaml"
ENV MINT_KEY="/opt/aptos/etc/mint.key"
ENV NODE_URL="http://127.0.0.1:8080"
ENV CHAIN_ID="TESTING"
ENV NODE_COUNT="1"
ENV NETWORK_IP="127.0.0.1"

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
