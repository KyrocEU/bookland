FROM node:22-bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
  ffmpeg fonts-dejavu-core fonts-noto-color-emoji \
  libnss3 libdbus-1-3 libatk1.0-0 libgbm1 libasound2 \
  libxrandr2 libxkbcommon0 libxfixes3 libxcomposite1 libxdamage1 \
  libatk-bridge2.0-0 libpango-1.0-0 libcairo2 libcups2 \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci && npx remotion browser ensure
COPY src ./src
COPY public ./public
COPY worker ./worker
RUN npx remotion bundle --out-dir=build
ENV HOST=0.0.0.0
ENV DATA_DIR=/data
ENV RENDER_CONCURRENCY=1
ENV REMOTION_BUNDLE=/app/build
ENV RENDER_LOW_MEMORY=1
EXPOSE 8787
CMD ["node", "worker/server.mjs"]
