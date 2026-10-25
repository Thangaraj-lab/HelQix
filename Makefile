# One entry point for the gates (K1). On Windows run the script from Git Bash: `bash scripts/check.sh`.
.PHONY: check dev-up dev-down

check:
	bash scripts/check.sh

dev-up:
	docker compose -f deploy/dev/compose.yaml --env-file deploy/dev/.env up -d --wait

dev-down:
	docker compose -f deploy/dev/compose.yaml --env-file deploy/dev/.env down
