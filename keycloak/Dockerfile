FROM quay.io/keycloak/keycloak:23.0.0

COPY realm-export.json /opt/keycloak/data/import/realm.json

# Enable usage of import feature
ENV KC_DB=postgres
# ENV KC_FEATURES=import  <-- This was invalid

# Start Keycloak, import the realm, and listen on http
CMD ["start-dev", "--import-realm"]
