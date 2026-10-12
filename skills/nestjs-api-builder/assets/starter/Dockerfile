FROM node:20-alpine AS builder

WORKDIR /usr/src/app

COPY package*.json pnpm-lock.yaml ./
RUN npm install -g pnpm@8 && pnpm install

COPY . .

RUN pnpm run build

FROM node:20-alpine

WORKDIR /usr/src/app

COPY --from=builder /usr/src/app/dist ./dist
COPY --from=builder /usr/src/app/node_modules ./node_modules

EXPOSE 8080

CMD ["node", "dist/main"]
