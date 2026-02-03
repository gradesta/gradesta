PORT ?= 8080

.PHONY: build run tidy

build:
	CGO_ENABLED=0 go build -o gradesta-file-browser

run:
	CGO_ENABLED=0 go run . -port $(PORT)

tidy:
	go mod tidy
