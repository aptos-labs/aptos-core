from dataclasses import asdict, replace
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from harness.config import ExperimentConfig
from harness.credentials import redact, redact_tree, require_provider_auth
from harness.model_profile import PROFILES, select_model, subscription_environment


ROOT = Path(__file__).resolve().parent.parent


class ModelProfileTest(unittest.TestCase):
    def setUp(self) -> None:
        self.config = replace(
            ExperimentConfig.load(ROOT / "config/default.json"),
            model=PROFILES["opus"][0], provider_base_url=PROFILES["opus"][1],
            effort="xhigh",
        )

    def test_select_preserves_limits_and_cannot_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "config.json"
            select_model(ROOT / "config/default.json", output, "opus")
            self.assertEqual(ExperimentConfig.load(output), self.config)
            with self.assertRaises(FileExistsError):
                select_model(output, output, "glm")
            self.assertEqual(ExperimentConfig.load(output), self.config)

    def test_subscription_clears_competing_authentication(self) -> None:
        original = {
            "CLAUDE_CODE_OAUTH_TOKEN": "test-oauth-secret",
            "ANTHROPIC_API_KEY": "test-api-secret",
            "ANTHROPIC_AUTH_TOKEN": "test-glm-secret",
            "ANTHROPIC_BASE_URL": PROFILES["glm"][1],
            "ANTHROPIC_DEFAULT_OPUS_MODEL": PROFILES["glm"][0],
            "CLAUDE_CODE_USE_FOUNDRY": "1", "ZAI_API_KEY": "test-zai-secret",
        }
        env = subscription_environment(self.config, original)
        self.assertEqual(env["ANTHROPIC_MODEL"], "claude-opus-5")
        self.assertEqual(env["CLAUDE_CODE_EFFORT_LEVEL"], "xhigh")
        self.assertEqual(env["ANTHROPIC_BASE_URL"], PROFILES["opus"][1])
        for name in ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN", "ANTHROPIC_DEFAULT_OPUS_MODEL", "CLAUDE_CODE_USE_FOUNDRY", "ZAI_API_KEY"):
            self.assertNotIn(name, env)
        with patch.dict(os.environ, env, clear=True):
            require_provider_auth(self.config.model, self.config.provider_base_url)

    def test_select_glm_restores_max_effort(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            opus = Path(directory) / "opus.json"
            glm = Path(directory) / "glm.json"
            select_model(ROOT / "config/default.json", opus, "opus")
            select_model(opus, glm, "glm")
            self.assertEqual(ExperimentConfig.load(glm).effort, "max")

    def test_effort_validation_preserves_history_and_rejects_glm_xhigh(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "config.json"
            path.write_text(json.dumps(asdict(replace(self.config, effort="max"))))
            self.assertEqual(ExperimentConfig.load(path).effort, "max")
            for model, effort in (("glm-5.3[1m]", "xhigh"), ("claude-opus-5", "typo")):
                path.write_text(json.dumps(asdict(replace(self.config, model=model, effort=effort))))
                with self.assertRaisesRegex(ValueError, "effort must"):
                    ExperimentConfig.load(path)

    def test_missing_subscription_never_falls_back_to_key(self) -> None:
        with self.assertRaisesRegex(ValueError, "subscription token missing"):
            subscription_environment(self.config, {"ANTHROPIC_API_KEY": "test-api"})

    def test_direct_launch_rejects_keys_even_with_oauth(self) -> None:
        for competing in ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"):
            with self.subTest(competing=competing), patch.dict(os.environ, {
                competing: "test-secret", "CLAUDE_CODE_OAUTH_TOKEN": "test-oauth",
            }, clear=True):
                with self.assertRaisesRegex(ValueError, "forbidden"):
                    require_provider_auth(self.config.model, self.config.provider_base_url)

    def test_direct_launch_requires_subscription_token(self) -> None:
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaisesRegex(ValueError, "no API fallback"):
                require_provider_auth(self.config.model, self.config.provider_base_url)

    def test_token_file_and_artifact_redaction(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            token = Path(directory) / "token"
            token.write_text("test-oauth-secret\n")
            env = subscription_environment(self.config, {"MOVE_INFERENCE_CLAUDE_TOKEN_FILE": str(token)})
            self.assertNotIn("MOVE_INFERENCE_CLAUDE_TOKEN_FILE", env)
            self.assertEqual(env["CLAUDE_CODE_OAUTH_TOKEN"], "test-oauth-secret")
            with patch.dict(os.environ, env, clear=True):
                self.assertEqual(redact({"nested": ["test-oauth-secret"]}), {"nested": ["[REDACTED]"]})
                redact_tree(Path(directory))
                self.assertNotIn("test-oauth-secret", token.read_text())


if __name__ == "__main__":
    unittest.main()
