docker build --build-arg GIT_COMMIT=$(git rev-parse HEAD) -t $(image_tag) -f service/Dockerfile .
