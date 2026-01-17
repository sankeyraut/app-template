# Stage 1: Template the realm configuration
FROM alpine:latest as templater

# Install envsubst
RUN apk add --no-cache gettext

# Copy the template and define build arguments
COPY realm-export.json /tmp/realm.json.template
ARG FRONTEND_URL
ARG REALM_NAME
ENV FRONTEND_URL=${FRONTEND_URL}
ENV REALM_NAME=${REALM_NAME}

# Run envsubst to create the temporary realm file
RUN envsubst < /tmp/realm.json.template > /tmp/realm.json

# Stage 2: Final Keycloak image
FROM quay.io/keycloak/keycloak:23.0.0

# Copy the templated realm file from the previous stage
COPY --from=templater /tmp/realm.json /opt/keycloak/data/import/realm.json

# Enable usage of import feature
ENV KC_DB=postgres

# Start Keycloak with the imported realm
ENTRYPOINT ["/opt/keycloak/bin/kc.sh", "start-dev", "--import-realm"]
