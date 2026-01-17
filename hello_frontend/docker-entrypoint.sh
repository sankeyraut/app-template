#!/bin/sh

# Generate env.js
echo "window._env_ = {" > /usr/share/nginx/html/env.js
echo "  KEYCLOAK_URL: \"${KEYCLOAK_URL}\"" >> /usr/share/nginx/html/env.js
echo "};" >> /usr/share/nginx/html/env.js

# Start nginx
exec "$@"
