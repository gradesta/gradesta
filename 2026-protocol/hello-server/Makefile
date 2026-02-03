PORT ?= 8081

.PHONY: build run tidy

build:
	CGO_ENABLED=0 go build -o gradesta-hello-server

run:
	CGO_ENABLED=0 go run . -port $(PORT)

tidy:
	go mod tidy
