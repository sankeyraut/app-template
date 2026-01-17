# Build Stage
FROM node:20-slim as build

WORKDIR /app

COPY package.json package-lock.json* ./
# install dependencies
RUN npm install

COPY . .
# Build the app to "dist" folder
RUN npm run build

# Serve Stage
FROM nginx:alpine

# Copy the build output
COPY --from=build /app/dist /usr/share/nginx/html

# Copy our nginx configuration
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Add entrypoint script
COPY docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

EXPOSE 80

ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["nginx", "-g", "daemon off;"]
