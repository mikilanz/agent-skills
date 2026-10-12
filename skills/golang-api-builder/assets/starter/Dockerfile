FROM golang:1.25-alpine AS build-stage

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download

COPY . .

RUN CGO_ENABLED=0 GOOS=linux go build -v -o /server ./cmd/api/main.go

FROM alpine:latest AS build-release-stage

WORKDIR /

COPY --from=build-stage /server /server
# migrations are read from file://internal/database/migrations relative to WORKDIR
COPY --from=build-stage /app/internal/database /internal/database

EXPOSE 8080

CMD [ "/server" ]
