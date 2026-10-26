# Card Place marketplace (web + mobile), static files served by demo-web-page/serve.mjs.
#   docker build -t card-place . && docker run -p 8080:8080 card-place
FROM node:22-alpine
WORKDIR /app
COPY demo-web-page ./demo-web-page
COPY mobile-app ./mobile-app
COPY docs ./docs
ENV PORT=8080
EXPOSE 8080
CMD ["node", "demo-web-page/serve.mjs"]
