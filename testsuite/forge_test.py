from contextlib import ExitStack
import json
import os
import textwrap
import unittest
import tempfile
from datetime import datetime, timezone, timedelta
from pathlib import Path
from typing import (
    Any,
    Dict,
    Protocol,
)
from unittest.mock import patch

import forge
from forge import (
    BEGIN_JUNIT,
    END_JUNIT,
    ForgeCluster,
    ForgeConfigBackend,
    ForgeContext,
    ForgeFormatter,
    ForgeJob,
    ForgeResult,
    ForgeState,
    K8sForgeRunner,
    LocalForgeRunner,
    SystemContext,
    ensure_provided_image_tags_has_profile_or_features,
    create_forge_command,
    find_recent_images,
    find_recent_images_by_profile_or_features,
    format_comment,
    format_junit_xml,
    format_pre_comment,
    format_report,
    get_all_forge_jobs,
    get_dashboard_link,
    get_humio_link_for_test_runner_logs,
    get_humio_link_for_node_logs,
    get_testsuite_images,
    main,
    sanitize_forge_resource_name,
    validate_forge_config,
    GAR_REPO_NAME,
)

from click.testing import CliRunner, Result
from test_framework.filesystem import FakeFilesystem, SpyFilesystem, FILE_NOT_FOUND
from test_framework.git import Git
from test_framework.process import FakeProcesses, SpyProcesses
from test_framework.cluster import (
    GetPodsItem,
    GetPodsItemMetadata,
    GetPodsItemStatus,
    GetPodsResult,
    list_eks_clusters,
    AwsListClusterResult,
)

from test_framework.shell import SpyShell, FakeShell, FakeCommand, RunResult
from test_framework.time import FakeTime
from test_framework.cluster import Cloud

# Show the entire diff when unittest fails assertion
unittest.util._MAX_LENGTH = 2000  # type: ignore


class HasAssertMultiLineEqual(Protocol):
    def assertMultiLineEqual(self, first: str, second: str, msg: Any = ...) -> None:
        ...


def get_cwd() -> Path:
    return Path(__file__).absolute().parent


def get_fixture_path(fixture_name: str) -> Path:
    return get_cwd() / "fixtures" / fixture_name


class AssertFixtureMixin:
    def assertFixture(
        self: HasAssertMultiLineEqual, test_str: str, fixture_name: str
    ) -> None:
        fixture = None
        fixture_path = get_fixture_path(fixture_name)
        if os.getenv("FORGE_WRITE_FIXTURES") == "true":
            print(f"Writing fixture to {str(fixture_path)}")
            fixture_path.write_text(test_str)
            fixture = test_str
        else:
            try:
                fixture = fixture_path.read_text()
            except FileNotFoundError as e:
                raise Exception(
                    f"Fixture {fixture_path} is missing.\nRun with FORGE_WRITE_FIXTURES=true to update the fixtures"
                ) from e
            except Exception as e:
                raise Exception(
                    f"Failed while reading fixture:\n{e}\nRun with FORGE_WRITE_FIXTURES=true to update the fixtures"
                ) from e
        temp = Path(tempfile.mkstemp()[1])
        temp.write_text(test_str)
        self.assertMultiLineEqual(
            test_str,
            fixture or "",
            f"Fixture {fixture_name} does not match"
            "\n"
            f"Wrote to {str(temp)} for comparison"
            "\nRerun with FORGE_WRITE_FIXTURES=true to update the fixtures",
        )


class FakeConfigBackend(ForgeConfigBackend):
    def __init__(self, store: object) -> None:
        self.store = store

    def create(self) -> None:
        pass

    def write(self, config: object) -> None:
        self.store = config

    def read(self) -> object:
        return self.store


def fake_context(
    shell=None,
    filesystem=None,
    processes=None,
    time=None,
    mode=None,
    multiregion=False,
) -> ForgeContext:
    return ForgeContext(
        shell=shell if shell else FakeShell(),
        filesystem=filesystem if filesystem else FakeFilesystem(),
        processes=processes if processes else FakeProcesses(),
        time=time if time else FakeTime(),
        forge_args=create_forge_command(
            forge_runner_mode=mode,
            forge_test_suite="banana",
            forge_runner_duration_secs="123",
            forge_num_validators="10",
            forge_num_validator_fullnodes="20",
            image_tag="asdf",
            upgrade_image_tag="upgrade_asdf",
            forge_namespace="forge-potato",
            forge_namespace_reuse="false",
            forge_namespace_keep="false",
            forge_enable_haproxy="false",
            forge_enable_indexer="false",
            forge_deployer_profile="",
            cargo_args=["--cargo-arg"],
            forge_cli_args=["--forge-cli-arg"],
            test_args=["--test-arg"],
        ),
        aws_account_num="123",
        aws_region="banana-east-1",
        forge_image_tag="forge_asdf",
        image_tag="asdf",
        upgrade_image_tag="upgrade_asdf",
        forge_namespace="forge-potato",
        forge_cluster=ForgeCluster(
            name="tomato", kubeconf="kubeconf", is_multiregion=multiregion
        ),
        forge_test_suite="banana",
        forge_username="banana-eater",
        forge_blocking=True,
        forge_retain_debug_logs="true",
        forge_junit_xml_path=None,
        github_actions="false",
        github_job_url="https://banana",
    )


class ForgeRunnerTests(unittest.TestCase):
    maxDiff = None

    def testLocalRunner(self) -> None:
        cargo_run = " ".join(
            [
                "cargo",
                "run",
                "--cargo-arg",
                "-p",
                "velor-forge-cli",
                "--",
                "--suite",
                "banana",
                "--duration-secs",
                "123",
                "--num-validators",
                "10",
                "--num-validator-fullnodes",
                "20",
                "--forge-cli-arg",
                "test",
                "k8s-swarm",
                "--image-tag",
                "asdf",
                "--upgrade-image-tag",
                "upgrade_asdf",
                "--namespace",
                "forge-potato",
                "--port-forward",
                "--test-arg",
            ]
        )
        shell = SpyShell(
            [
                FakeCommand(
                    cargo_run,
                    RunResult(0, b"orange"),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf get pods -n forge-potato",
                    RunResult(0, b"Pods"),
                ),
            ]
        )
        filesystem = SpyFilesystem({}, {})
        context = fake_context(shell, filesystem, mode="local")
        runner = LocalForgeRunner()
        result = runner.run(context)
        self.assertEqual(result.state, ForgeState.PASS, result.output)
        shell.assert_commands(self)
        filesystem.assert_writes(self)
        filesystem.assert_reads(self)

    def testK8sRunner(self) -> None:
        self.maxDiff = None
        shell = SpyShell(
            [
                FakeCommand(
                    "kubectl --kubeconfig kubeconf delete pod -n default -l forge-namespace=forge-potato --force",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf wait -n default --for=delete pod -l forge-namespace=forge-potato",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf apply -n default -f temp1",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf wait -n default --timeout=5m --for=condition=Ready pod/forge-potato-1659078000-asdf",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf logs -n default -f forge-potato-1659078000-asdf",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf get pod -n default forge-potato-1659078000-asdf -o jsonpath='{.status.phase}'",
                    RunResult(0, b"Succeeded"),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf get pods -n forge-potato",
                    RunResult(0, b"Pods"),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf delete pod -n default -l forge-namespace=forge-potato --force",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf wait -n default --for=delete pod -l forge-namespace=forge-potato",
                    RunResult(0, b""),
                ),
            ]
        )
        forge_yaml = get_cwd() / "forge-test-runner-template.yaml"
        template_fixture = get_fixture_path("forge-test-runner-template.fixture")
        filesystem = SpyFilesystem(
            {
                "temp1": template_fixture.read_bytes(),
            },
            {
                "forge-test-runner-template.yaml": FILE_NOT_FOUND,
                "testsuite/forge-test-runner-template.yaml": forge_yaml.read_bytes(),
            },
        )
        context = fake_context(shell, filesystem, mode="k8s")
        runner = K8sForgeRunner()
        result = runner.run(context)
        shell.assert_commands(self)
        filesystem.assert_writes(self)
        filesystem.assert_reads(self)
        self.assertEqual(result.state, ForgeState.PASS, result.output)

    def testK8sRunnerWithMultiregionCluster(self) -> None:
        self.maxDiff = None
        shell = SpyShell(
            [
                FakeCommand(
                    "kubectl --kubeconfig kubeconf --context=karmada-apiserver delete pod -n default -l forge-namespace=forge-potato --force",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf wait -n default --for=delete pod -l forge-namespace=forge-potato",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf --context=karmada-apiserver apply -n default -f temp1",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf wait -n default --timeout=5m --for=condition=Ready pod/forge-potato-1659078000-asdf",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf logs -n default -f forge-potato-1659078000-asdf",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf get pod -n default forge-potato-1659078000-asdf -o jsonpath='{.status.phase}'",
                    RunResult(0, b"Succeeded"),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf get pods -n forge-potato",
                    RunResult(0, b"Pods"),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf --context=karmada-apiserver delete pod -n default -l forge-namespace=forge-potato --force",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig kubeconf wait -n default --for=delete pod -l forge-namespace=forge-potato",
                    RunResult(0, b""),
                ),
            ]
        )
        forge_yaml = get_cwd() / "forge-test-runner-template.yaml"
        template_fixture = get_fixture_path("forge-test-runner-template.fixture")
        filesystem = SpyFilesystem(
            {
                "temp1": template_fixture.read_bytes(),
            },
            {
                "forge-test-runner-template.yaml": FILE_NOT_FOUND,
                "testsuite/forge-test-runner-template.yaml": forge_yaml.read_bytes(),
            },
        )
        context = fake_context(shell, filesystem, mode="k8s", multiregion=True)
        runner = K8sForgeRunner()
        result = runner.run(context)
        shell.assert_commands(self)
        filesystem.assert_writes(self)
        filesystem.assert_reads(self)
        self.assertEqual(result.state, ForgeState.PASS, result.output)


class TestFindRecentImage(unittest.TestCase):
    def testFindRecentImage(self) -> None:
        shell = SpyShell(
            [
                FakeCommand("git rev-parse HEAD~0", RunResult(0, b"potato\n")),
                FakeCommand(
                    "aws ecr describe-images --repository-name velor/validator-testing --image-ids imageTag=potato",
                    RunResult(1, b""),
                ),
                FakeCommand("git rev-parse HEAD~1", RunResult(0, b"lychee\n")),
                FakeCommand(
                    "aws ecr describe-images --repository-name velor/validator-testing --image-ids imageTag=lychee",
                    RunResult(0, b""),
                ),
            ]
        )
        git = Git(shell)
        image_tags = find_recent_images(
            shell, git, 1, "validator-testing", cloud=Cloud.AWS
        )
        self.assertEqual(list(image_tags), ["lychee"])
        shell.assert_commands(self)

    def testFindRecentImageGcp(self) -> None:
        shell = SpyShell(
            [
                FakeCommand("git rev-parse HEAD~0", RunResult(0, b"potato\n")),
                FakeCommand(
                    f"crane manifest {GAR_REPO_NAME}/validator-testing:potato",
                    RunResult(1, b""),
                ),
                FakeCommand("git rev-parse HEAD~1", RunResult(0, b"lychee\n")),
                FakeCommand(
                    f"crane manifest {GAR_REPO_NAME}/validator-testing:lychee",
                    RunResult(0, b""),
                ),
            ]
        )
        git = Git(shell)
        image_tags = find_recent_images(
            shell, git, 1, "validator-testing", cloud=Cloud.GCP
        )
        self.assertEqual(list(image_tags), ["lychee"])
        shell.assert_commands(self)

    def testFindRecentFailpointsImage(self) -> None:
        shell = SpyShell(
            [
                FakeCommand("git rev-parse HEAD~0", RunResult(0, b"tomato\n")),
                FakeCommand(
                    "aws ecr describe-images --repository-name velor/validator-testing --image-ids imageTag=failpoints_tomato",
                    RunResult(0, b""),
                ),
            ]
        )
        git = Git(shell)
        image_tags = find_recent_images_by_profile_or_features(
            shell,
            git,
            1,
            enable_performance_profile=False,
            enable_failpoints=True,
            cloud=Cloud.AWS,
        )
        self.assertEqual(list(image_tags), ["failpoints_tomato"])
        shell.assert_commands(self)

    def testFindRecentPerformanceImage(self) -> None:
        shell = SpyShell(
            [
                FakeCommand("git rev-parse HEAD~0", RunResult(0, b"potato\n")),
                FakeCommand(
                    "aws ecr describe-images --repository-name velor/validator-testing --image-ids imageTag=performance_potato",
                    RunResult(0, b""),
                ),
            ]
        )
        git = Git(shell)
        image_tags = find_recent_images_by_profile_or_features(
            shell,
            git,
            1,
            enable_performance_profile=True,
            enable_failpoints=False,
            cloud=Cloud.AWS,
        )
        self.assertEqual(list(image_tags), ["performance_potato"])
        shell.assert_commands(self)

    def testFailBothFailpointsPerformance(self) -> None:
        shell = SpyShell([])
        git = Git(shell)
        with self.assertRaises(Exception):
            find_recent_images_by_profile_or_features(
                shell,
                git,
                1,
                enable_performance_profile=True,
                enable_failpoints=True,
            )

    def testDidntFindRecentImage(self) -> None:
        shell = SpyShell(
            [
                FakeCommand("git rev-parse HEAD~0", RunResult(0, b"crab\n")),
                FakeCommand(
                    "aws ecr describe-images --repository-name velor/validator-testing --image-ids imageTag=crab",
                    RunResult(1, b""),
                ),
            ]
        )
        git = Git(shell)
        with self.assertRaises(Exception):
            list(
                find_recent_images(
                    shell, git, 1, "velor/validator-testing", commit_threshold=1
                )
            )

    def testFindRecentFewImages(
        self,
    ) -> None:  # such as in compat test where we find 2 images
        shell = SpyShell(
            [
                FakeCommand("git rev-parse HEAD~0", RunResult(0, b"crab\n")),
                FakeCommand(
                    "aws ecr describe-images --repository-name velor/validator --image-ids imageTag=crab",
                    RunResult(0, b""),
                ),
                FakeCommand("git rev-parse HEAD~1", RunResult(0, b"shrimp\n")),
                FakeCommand(
                    "aws ecr describe-images --repository-name velor/validator --image-ids imageTag=shrimp",
                    RunResult(0, b""),
                ),
            ]
        )
        git = Git(shell)
        images = find_recent_images(shell, git, 2, "validator", cloud=Cloud.AWS)
        self.assertEqual(list(images), ["crab", "shrimp"])

    def testFailpointsProvidedImageTag(self) -> None:
        tag1, tag2 = ensure_provided_image_tags_has_profile_or_features(
            "potato_tomato",
            "failpoints_performance_potato",
            enable_failpoints=True,
            enable_performance_profile=False,
        )
        self.assertEqual(tag1, "failpoints_potato_tomato")  # it's added
        self.assertEqual(tag2, "failpoints_performance_potato")  # no change

    def testPerformaneProfilePartialProvidedImageTag(self) -> None:
        tag1, tag2 = ensure_provided_image_tags_has_profile_or_features(
            "potato_tomato",
            None,
            enable_failpoints=False,
            enable_performance_profile=True,
        )
        self.assertEqual(tag1, "performance_potato_tomato")  # it's added
        self.assertIsNone(tag2)

    def testFailpointsNoProvidedImageTag(self) -> None:
        tag1, tag2 = ensure_provided_image_tags_has_profile_or_features(
            None,
            None,
            enable_failpoints=True,
            enable_performance_profile=False,
        )
        self.assertIsNone(tag1)
        self.assertIsNone(tag2)


class ForgeFormattingTests(unittest.TestCase, AssertFixtureMixin):
    maxDiff = None

    def testTestsuiteImagesSameImage(self) -> None:
        context = fake_context()
        context.upgrade_image_tag = context.image_tag
        txt = get_testsuite_images(context)
        self.assertEqual(txt, f"`asdf`")

    def testTestsuiteImagesUpgrade(self) -> None:
        context = fake_context()
        txt = get_testsuite_images(context)
        self.assertEqual(
            txt,
            f"`asdf` ==> `upgrade_asdf`",
        )

    def testReport(self) -> None:
        filesystem = SpyFilesystem({"test": b"banana"}, {})
        context = fake_context(filesystem=filesystem)
        result = ForgeResult.from_args(ForgeState.PASS, "test")
        context.report(result, [ForgeFormatter("test", lambda c, r: "banana")])
        filesystem.assert_reads(self)
        filesystem.assert_writes(self)

    def testGetHumioLogsLinkRelative(self) -> None:
        link = get_humio_link_for_node_logs("forge-pr-2983", True)
        self.assertIn("forge-pr-2983", link)
        self.assertFixture(link, "testGetHumioLogsLinkRelative.fixture")

    def testGetHumioLogsLinkAbsolute(self) -> None:
        time = FakeTime()
        link = get_humio_link_for_node_logs("forge-pr-2984", (time.now(), time.now()))
        self.assertIn("forge-pr-2984", link)
        self.assertFixture(link, "testGetHumioLogsLinkAbsolute.fixture")

    def testGetHumioForgeLinkRelative(self) -> None:
        link = get_humio_link_for_test_runner_logs("forge-pr-2985", True)
        self.assertIn("forge-pr-2985", link)
        self.assertFixture(link, "testGetHumioForgeLinkRelative.fixture")

    def testGetHumioForgeLinkAbsolute(self) -> None:
        link = get_humio_link_for_test_runner_logs("forge-pr-2986", True)
        self.assertIn("forge-pr-2986", link)
        self.assertFixture(link, "testGetHumioForgeLinkAbsolute.fixture")

    def testDashboardLinkAutoRefresh(self) -> None:
        self.assertFixture(
            get_dashboard_link(
                "forge-pr-2983",
                # Chain names don't use the "velor-" prefix.
                "forge-big-1",
                True,
            ),
            "testDashboardLinkAutoRefresh.fixture",
        )

    def testDashboardLinkTimeInterval(self) -> None:
        self.assertFixture(
            get_dashboard_link(
                "forge-pr-2983",
                # Chain names don't use the "velor-" prefix.
                "forge-big-1",
                (
                    datetime.fromtimestamp(100000, timezone.utc),
                    datetime.fromtimestamp(100001, timezone.utc),
                ),
            ),
            "testDashboardLinkTimeInterval.fixture",
        )

    def testFormatPreComment(self) -> None:
        context = fake_context()
        pre_comment = format_pre_comment(context)
        self.maxDiff = 10
        self.assertIn(
            "var-namespace=forge-potato",
            pre_comment,
            "Wrong forge namespace in pre comment",
        )
        self.assertFixture(pre_comment, "testFormatPreComment.fixture")

    def testFormatComment(self) -> None:
        context = fake_context()
        report_fixture = get_fixture_path("report.fixture")
        with ForgeResult.with_context(context) as forge_result:
            forge_result.set_state(ForgeState.PASS)
            forge_result.set_output(report_fixture.read_text())
        forge_comment = format_comment(context, forge_result)
        self.assertIn(
            "var-namespace=forge-potato",
            forge_comment,
            "Wrong forge namespace in comment",
        )
        self.assertFixture(forge_comment, "testFormatComment.fixture")

    def testFormatReport(self) -> None:
        context = fake_context()
        report_fixture = get_fixture_path("report.fixture")
        with ForgeResult.with_context(context) as forge_result:
            forge_result.set_state(ForgeState.PASS)
            forge_result.set_output(report_fixture.read_text())
        self.assertFixture(
            format_report(context, forge_result),
            "testFormatReport.fixture",
        )

    def testSanitizeForgeNamespaceLastCharacter(self) -> None:
        namespace_with_invalid_last_char = "forge-$$$"
        namespace = sanitize_forge_resource_name(namespace_with_invalid_last_char)
        self.assertEqual(namespace, "forge---0")

    def testSanitizeForgeNamespaceSlashes(self) -> None:
        namespace_with_slash = "forge-banana/apple"
        namespace = sanitize_forge_resource_name(namespace_with_slash)
        self.assertEqual(namespace, "forge-banana-apple")

    def testSanitizeForgeNamespaceStartsWith(self) -> None:
        namespace_with_invalid_start = "frog-"
        self.assertRaises(
            Exception, sanitize_forge_resource_name, namespace_with_invalid_start
        )

    def testSanitizeForgeNamespaceTooLong(self) -> None:
        namespace_too_long = "forge-" + "a" * 10000
        namespace = sanitize_forge_resource_name(namespace_too_long)
        self.assertEqual(
            namespace,
            "forge-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )

    def testPossibleAuthFailureMessage(self) -> None:
        result = ForgeResult.empty()
        context = fake_context()
        now = context.time.now()
        result._start_time = now - timedelta(seconds=4800)
        result._end_time = now
        result.state = ForgeState.FAIL
        output = result.format(context)
        self.assertFixture(output, "testPossibleAuthFailureMessage.fixture")

    def testFormatJunitXml(self) -> None:
        result = ForgeResult.empty()
        context = fake_context()

        result.set_output(
            textwrap.dedent(
                f"""
        {BEGIN_JUNIT}
        <testsuites>
        blah
        </testsuites>
        {END_JUNIT}
        """
            )
        )

        output = format_junit_xml(context, result)
        self.assertFixture(output, "testFormatJunitXml.fixture")


class ForgeMainTests(unittest.TestCase, AssertFixtureMixin):
    maxDiff = None

    def testMain(self) -> None:
        runner = CliRunner()
        shell = SpyShell(
            [
                FakeCommand(
                    "aws sts get-caller-identity",
                    RunResult(0, b'{"Account": "123456789012"}'),
                ),
                FakeCommand(
                    "aws eks list-clusters",
                    RunResult(0, b'{ "clusters": [ "velor-forge-big-1" ] }'),
                ),
                FakeCommand(
                    # NOTE: with multi-cloud support, we set the kubeconfig to ensure auth before continuing
                    # See changes in: https://github.com/velor-chain/velor-core/pull/6166
                    "aws eks update-kubeconfig --name velor-forge-big-1 --kubeconfig temp1",
                    RunResult(0, b""),
                ),
                FakeCommand("git rev-parse HEAD~0", RunResult(0, b"banana")),
                FakeCommand(
                    "aws ecr describe-images --repository-name velor/validator-testing "
                    "--image-ids imageTag=banana",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "aws ecr describe-images --repository-name velor/validator-testing "
                    "--image-ids imageTag=banana",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "aws ecr describe-images --repository-name velor/validator-testing "
                    "--image-ids imageTag=banana",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "aws ecr describe-images --repository-name velor/forge --image-ids "
                    "imageTag=banana",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig temp1 delete pod -n default -l forge-namespace=forge-perry-1659078000 "
                    "--force",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig temp1 wait -n default --for=delete pod -l "
                    "forge-namespace=forge-perry-1659078000",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig temp1 apply -n default -f temp2",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig temp1 wait -n default --timeout=5m --for=condition=Ready "
                    "pod/forge-perry-1659078000-1659078000-banana",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig temp1 logs -n default -f forge-perry-1659078000-1659078000-banana",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig temp1 get pod -n default forge-perry-1659078000-1659078000-banana -o "
                    "jsonpath='{.status.phase}'",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig temp1 get pods -n forge-perry-1659078000",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig temp1 delete pod -n default -l forge-namespace=forge-perry-1659078000 "
                    "--force",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl --kubeconfig temp1 wait -n default --for=delete pod -l "
                    "forge-namespace=forge-perry-1659078000",
                    RunResult(0, b""),
                ),
            ]
        )
        filesystem = SpyFilesystem(
            {
                "temp-comment": get_fixture_path(
                    "testMainComment.fixture"
                ).read_bytes(),
                "temp-step-summary": get_fixture_path(
                    "testMainComment.fixture"
                ).read_bytes(),
                "temp-pre-comment": get_fixture_path(
                    "testMainPreComment.fixture"
                ).read_bytes(),
                "temp-report": get_fixture_path("testMainReport.fixture").read_bytes(),
            },
            {},
        )
        with ExitStack() as stack:
            stack.enter_context(runner.isolated_filesystem())
            stack.enter_context(
                patch.object(forge, "LocalFilesystem", lambda: filesystem)
            )
            stack.enter_context(patch.object(forge, "LocalShell", lambda *_: shell))
            stack.enter_context(patch.object(forge, "SystemTime", lambda: FakeTime()))
            stack.enter_context(
                patch.object(forge, "SystemProcesses", lambda: FakeProcesses())
            )
            stack.enter_context(
                patch.object(
                    forge,
                    "S3ForgeConfigBackend",
                    lambda *_: FakeConfigBackend(
                        {
                            "enabled_clusters": ["velor-forge-big-1"],
                            "all_clusters": ["velor-forge-big-1", "banana"],
                            "test_suites": {},
                        }
                    ),
                )
            )

            os.mkdir(".git")
            os.mkdir("testsuite")
            template_name = "forge-test-runner-template.yaml"
            Path(f"testsuite/{template_name}").write_text(
                (Path(__file__).parent / template_name).read_text()
            )
            result = runner.invoke(
                main,
                [
                    "--no-log-metadata",
                    "test",
                    "--forge-cluster-name",
                    "velor-forge-big-1",
                    "--forge-report",
                    "temp-report",
                    "--forge-pre-comment",
                    "temp-pre-comment",
                    "--forge-comment",
                    "temp-comment",
                    "--github-step-summary",
                    "temp-step-summary",
                    "--github-server-url",
                    "None",
                    "--github-repository",
                    "None",
                    "--github-run-id",
                    "None",
                    "banana-test",
                ],
                catch_exceptions=False,
            )
            shell.assert_commands(self)
            self.assertFixture(
                filesystem.get_write("temp-comment").decode(), "testMainComment.fixture"
            )
            self.assertFixture(
                filesystem.get_write("temp-step-summary").decode(),
                "testMainComment.fixture",
            )
            self.assertFixture(
                filesystem.get_write("temp-pre-comment").decode(),
                "testMainPreComment.fixture",
            )
            self.assertFixture(
                filesystem.get_write("temp-report").decode(), "testMainReport.fixture"
            )
            self.assertFixture(result.output, "testMain.fixture")


class TestListClusters(unittest.TestCase):
    def testListClusters(self) -> None:
        fake_clusters = json.dumps(
            AwsListClusterResult(
                clusters=[
                    "banana-fake-1",
                    "velor-forge-banana-1",
                    "velor-forge-potato-2",
                ]
            ),
        )
        shell = SpyShell(
            [
                FakeCommand(
                    "aws eks list-clusters", RunResult(0, fake_clusters.encode())
                ),
            ]
        )
        clusters = list(list_eks_clusters(shell).keys())
        self.assertEqual(clusters, ["velor-forge-banana-1", "velor-forge-potato-2"])
        shell.assert_commands(self)

    def testListClustersFails(self) -> None:
        with self.assertRaises(Exception):
            shell = SpyShell(
                [
                    FakeCommand("Blah", RunResult(0, b"")),
                ]
            )
            list_eks_clusters(shell)
            shell.assert_commands(self)


def fake_pod_item(name: str, phase: str, labels: Dict = {}) -> GetPodsItem:
    return GetPodsItem(
        metadata=GetPodsItemMetadata(name=name, labels=labels),
        status=GetPodsItemStatus(phase=phase),
    )


class GetForgeJobsTests(unittest.IsolatedAsyncioTestCase):
    maxDiff = None

    async def testGetAllForgeJobs(self) -> None:
        fake_clusters = ["velor-forge-banana", "velor-forge-apple-2"]

        # The first set of test runner pods and their test pods
        fake_first_pods = GetPodsResult(
            items=[
                fake_pod_item(
                    "forge-first", "Running", labels={"forge-namespace": "forge-first"}
                ),
                fake_pod_item(
                    "forge-failed", "Failed", labels={"forge-namespace": "forge-failed"}
                ),
                fake_pod_item(
                    "ignore-me", "Failed", labels={"forge-namespace": "ignore-me"}
                ),
            ]
        )
        fake_forge_first_first_cluster_pods = GetPodsResult(
            items=[
                fake_pod_item("velor-node-0-validator", "Running"),
                fake_pod_item("velor-node-1-validator", "Running"),
            ]
        )
        fake_forge_first_failed_cluster_pods = GetPodsResult(
            items=[
                fake_pod_item("velor-node-0-validator", "Running"),
                fake_pod_item("velor-node-1-validator", "Running"),
                fake_pod_item("velor-node-0-fullnode", "Running"),
                fake_pod_item("velor-node-1-fullnode", "Running"),
            ]
        )
        fake_forge_first_ignore_me_cluster_pods = GetPodsResult(
            items=[
                fake_pod_item("velor-node-0-validator", "Failed"),
                fake_pod_item("velor-node-1-validator", "Running"),
            ]
        )

        # The second set of test runner pods and their test pods
        fake_second_pods = GetPodsResult(
            items=[
                fake_pod_item(
                    "forge-second",
                    "Running",
                    labels={"forge-namespace": "forge-second"},
                ),
                fake_pod_item(
                    "forge-succeeded",
                    "Succeeded",
                    labels={"forge-namespace": "forge-succeeded"},
                ),
                fake_pod_item("me-too", "Failed", labels={"forge-namespace": "me-too"}),
            ]
        )
        fake_forge_second_second_cluster_pods = GetPodsResult(
            items=[
                fake_pod_item("velor-node-0-validator", "Running"),
                fake_pod_item("velor-node-1-fullnode", "Running"),
            ]
        )
        fake_forge_second_succeeded_cluster_pods = GetPodsResult(
            items=[]  # succeeded, so there might be no pods left in its namespace
        )
        fake_forge_second_me_too_cluster_pods = GetPodsResult(
            items=[]  # failed, so there might be no pods left in its namespace
        )
        shell = SpyShell(
            [
                FakeCommand(
                    "aws eks update-kubeconfig --name velor-forge-banana --kubeconfig temp1",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl get pods -n default --kubeconfig temp1 -o json",
                    RunResult(0, json.dumps(fake_first_pods).encode()),
                ),
                FakeCommand(
                    "kubectl get pods -n forge-first --kubeconfig temp1 -o json",
                    RunResult(
                        0, json.dumps(fake_forge_first_first_cluster_pods).encode()
                    ),
                ),
                FakeCommand(
                    "kubectl get pods -n forge-failed --kubeconfig temp1 -o json",
                    RunResult(
                        0, json.dumps(fake_forge_first_failed_cluster_pods).encode()
                    ),
                ),
                FakeCommand(
                    "kubectl get pods -n ignore-me --kubeconfig temp1 -o json",
                    RunResult(
                        0, json.dumps(fake_forge_first_ignore_me_cluster_pods).encode()
                    ),
                ),
                FakeCommand(
                    "aws eks update-kubeconfig --name velor-forge-apple-2 --kubeconfig temp2",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "kubectl get pods -n default --kubeconfig temp2 -o json",
                    RunResult(0, json.dumps(fake_second_pods).encode()),
                ),
                FakeCommand(
                    "kubectl get pods -n forge-second --kubeconfig temp2 -o json",
                    RunResult(
                        0, json.dumps(fake_forge_second_second_cluster_pods).encode()
                    ),
                ),
                FakeCommand(
                    "kubectl get pods -n forge-succeeded --kubeconfig temp2 -o json",
                    RunResult(
                        0, json.dumps(fake_forge_second_succeeded_cluster_pods).encode()
                    ),
                ),
                FakeCommand(
                    "kubectl get pods -n me-too --kubeconfig temp2 -o json",
                    RunResult(
                        0, json.dumps(fake_forge_second_me_too_cluster_pods).encode()
                    ),
                ),
            ],
            strict=True,
        )
        filesystem = SpyFilesystem({}, {}, ["temp1", "temp2"])
        processes = SpyProcesses()
        context = SystemContext(shell, filesystem, processes, FakeTime())
        jobs = await get_all_forge_jobs(context, fake_clusters)
        expected_jobs = [
            ForgeJob(
                name="forge-first",
                phase="Running",
                cluster=ForgeCluster(
                    name="velor-forge-banana",
                    kubeconf="temp1",
                ),
                num_validators=2,
            ),
            ForgeJob(
                name="forge-failed",
                phase="Failed",
                cluster=ForgeCluster(
                    name="velor-forge-banana",
                    kubeconf="temp1",
                ),
                num_validators=2,
                num_fullnodes=2,
            ),
            ForgeJob(
                name="forge-second",
                phase="Running",
                cluster=ForgeCluster(
                    name="velor-forge-apple-2",
                    kubeconf="temp2",
                ),
                num_validators=1,
                num_fullnodes=1,
            ),
            ForgeJob(
                name="forge-succeeded",
                phase="Succeeded",
                cluster=ForgeCluster(
                    name="velor-forge-apple-2",
                    kubeconf="temp2",
                ),
            ),
        ]
        self.assertEqual(jobs, expected_jobs)
        processes.run_atexit()
        filesystem.assert_unlinks(self)


class ForgeConfigTests(unittest.TestCase):
    maxDiff = None

    def testCreate(self) -> None:
        runner = CliRunner()
        shell = SpyShell(
            [
                FakeCommand("aws s3 mb s3://forge-wrapper-config", RunResult(0, b"")),
            ]
        )
        with patch.object(forge, "LocalShell", lambda: shell):
            result = runner.invoke(
                main,
                ["config", "create"],
                catch_exceptions=False,
            )
            shell.assert_commands(self)
            self.assertEqual(result.exit_code, 0)

    def testValidateInvalidConfig(self) -> None:
        self.assertEqual(
            validate_forge_config({}),
            [
                "Missing required field enabled_clusters",
                "Missing required field all_clusters",
            ],
        )

    def testValidateValidConfig(self) -> None:
        self.assertEqual(
            validate_forge_config(
                {
                    "enabled_clusters": ["banana"],
                    "all_clusters": ["banana", "apple"],
                }
            ),
            [],
        )

    def testValidateValidHelmConfig(self) -> None:
        self.assertEqual(
            validate_forge_config(
                {
                    "enabled_clusters": ["banana"],
                    "all_clusters": ["banana", "apple"],
                    "default_helm_values": {
                        "velor-node": {"image": {"tag": "banana"}},
                        "velor-genesis": {"image": {"tag": "banana"}},
                    },
                }
            ),
            [],
        )

    def testValidateInvalidHelmConfig(self) -> None:
        self.assertEqual(
            validate_forge_config(
                {
                    "enabled_clusters": ["banana"],
                    "all_clusters": ["banana", "apple"],
                    "default_helm_values": {
                        "apple": "enabled",
                        "banana": {"enabled": "true"},
                    },
                }
            ),
            [],
        )

    def testValidateMissingClusterConfig(self) -> None:
        self.assertEqual(
            validate_forge_config(
                {
                    "enabled_clusters": ["apple"],
                    "all_clusters": ["banana", "potato"],
                }
            ),
            [],
        )

    def testHelmGetConfig(self) -> None:
        helm_before = {
            "enabled_clusters": ["banana"],
            "all_clusters": ["banana", "apple"],
        }
        helm_after_missing = {
            "enabled_clusters": ["banana"],
            "all_clusters": ["banana", "apple"],
            "default_helm_values": {
                "velor-node": {"apple": "enabled", "banana": {"enabled": "true"}}
            },
        }
        helm_after_complete = {
            "enabled_clusters": ["banana"],
            "all_clusters": ["banana", "apple"],
            "default_helm_values": {
                "velor-node": {"apple": "enabled", "banana": {"enabled": "true"}},
                "velor-genesis": {"apple": "enabled", "banana": {"enabled": "true"}},
            },
        }
        runner = CliRunner()
        shell = SpyShell(
            [
                FakeCommand(
                    "aws s3api get-object --bucket forge-wrapper-config --key "
                    "forge-wrapper-config.json temp1",
                    RunResult(0, json.dumps(helm_before).encode("utf-8")),
                ),
                FakeCommand(
                    "aws s3api get-object --bucket forge-wrapper-config --key "
                    "forge-wrapper-config.json temp2",
                    RunResult(0, json.dumps(helm_after_missing).encode("utf-8")),
                ),
                FakeCommand(
                    "aws s3api get-object --bucket forge-wrapper-config --key "
                    "forge-wrapper-config.json temp3",
                    RunResult(0, json.dumps(helm_after_complete).encode("utf-8")),
                ),
            ]
        )

        filesystem = SpyFilesystem(
            {},
            {
                "temp1": json.dumps(helm_before).encode(),
                "temp2": json.dumps(helm_after_missing).encode(),
                "temp3": json.dumps(helm_after_complete).encode(),
            },
        )
        with ExitStack() as stack:
            stack.enter_context(patch.object(forge, "LocalShell", lambda: shell))
            stack.enter_context(
                patch.object(forge, "LocalFilesystem", lambda: filesystem)
            )
            result_helm_config_not_present: Result = runner.invoke(
                main,
                ["--no-log-metadata", "config", "helm", "get", "velor-node"],
                catch_exceptions=True,
            )
            result_helm_config_present_missing = runner.invoke(
                main,
                ["--no-log-metadata", "config", "helm", "get", "velor-genesis"],
                catch_exceptions=True,
            )
            result_helm_config_present_complete = runner.invoke(
                main,
                ["--no-log-metadata", "config", "helm", "get", "velor-node"],
                catch_exceptions=True,
            )
            # assert all commands and filesystem calls are correct
            shell.assert_commands(self)
            filesystem.assert_reads(self)
            filesystem.assert_writes(self)

            # assert that we error with a message when the config is not present
            self.assertEqual(result_helm_config_not_present.exit_code, 1)
            self.assertIsNotNone(result_helm_config_not_present.exception)
            self.assertEqual(
                result_helm_config_not_present.exception.args,  # type: ignore
                Exception("Missing key default_helm_values in Forge config").args,
            )

            # assert that we error with a message when the config is missing partial information
            self.assertEqual(result_helm_config_present_missing.exit_code, 1)
            self.assertIsNotNone(result_helm_config_present_missing.exception)
            self.assertEqual(
                result_helm_config_present_missing.exception.args,  # type: ignore
                Exception("No helm values found for chart velor-genesis").args,
            )

            # we successfully get the config
            self.assertEqual(result_helm_config_present_complete.exit_code, 0)
            self.assertIsNotNone(helm_after_complete.get("default_helm_values"))
            self.assertIsNotNone(helm_after_complete.get("default_helm_values").get("velor-node"))  # type: ignore
            # the output config is printed with an extra newline
            self.assertEqual(
                result_helm_config_present_complete.stdout_bytes,
                f'{json.dumps(helm_after_complete.get("default_helm_values").get("velor-node"), indent=2)}\n'.encode(),  # type: ignore
            )

    def testHelmSetConfig(self) -> None:
        runner = CliRunner()
        shell = SpyShell(
            [
                FakeCommand(
                    "aws s3api get-object --bucket forge-wrapper-config --key "
                    "forge-wrapper-config.json temp1",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "aws s3api put-object --bucket forge-wrapper-config --key "
                    "forge-wrapper-config.json --body temp2",
                    RunResult(0, b""),
                ),
            ]
        )
        config_before = {
            "enabled_clusters": ["banana"],
            "all_clusters": ["banana", "apple"],
            "default_helm_values": {
                "velor-node": {"apple": "enabled", "banana": {"enabled": "false"}}
            },
        }
        config_after = {
            **config_before,
            "default_helm_values": {
                "velor-node": {"apple": "enabled", "banana": {"enabled": "true"}}
            },
        }
        filesystem = SpyFilesystem(
            {
                # new config which merges old config and new helm config written to temp file before pushing to s3
                "temp2": json.dumps(config_after).encode(),
            },
            {
                # read old config that has been written by s3 CLI
                "temp1": json.dumps(config_before).encode(),
                # read the new *helm* config from disk
                "temp2": json.dumps(
                    config_after["default_helm_values"]["velor-node"]
                ).encode(),
            },
        )
        with ExitStack() as stack:
            stack.enter_context(patch.object(forge, "LocalShell", lambda: shell))
            stack.enter_context(
                patch.object(forge, "LocalFilesystem", lambda: filesystem)
            )
            ret = runner.invoke(
                main,
                ["config", "helm", "set", "velor-node", "--config", "temp2", "-y"],
                catch_exceptions=True,
            )
            shell.assert_commands(self)
            filesystem.assert_reads(self)
            filesystem.assert_writes(self)

    def testHelmSetNewConfig(self) -> None:
        runner = CliRunner()
        shell = SpyShell(
            [
                FakeCommand(
                    "aws s3api get-object --bucket forge-wrapper-config --key "
                    "forge-wrapper-config.json temp1",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "aws s3api put-object --bucket forge-wrapper-config --key "
                    "forge-wrapper-config.json --body temp2",
                    RunResult(0, b""),
                ),
            ]
        )
        config_before = {
            "enabled_clusters": ["banana"],
            "all_clusters": ["banana", "apple"],
            "default_helm_values": {},
        }
        config_after = {
            **config_before,
            "default_helm_values": {
                "velor-node": {"apple": "enabled", "banana": {"enabled": "true"}}
            },
        }
        filesystem = SpyFilesystem(
            {
                # new config which merges old config and new helm config written to temp file before pushing to s3
                "temp2": json.dumps(config_after).encode(),
            },
            {
                # read old config that has been written by s3 CLI
                "temp1": json.dumps(config_before).encode(),
                # read the new *helm* config from disk
                "temp2": json.dumps(
                    config_after["default_helm_values"]["velor-node"]
                ).encode(),
            },
        )
        with ExitStack() as stack:
            stack.enter_context(patch.object(forge, "LocalShell", lambda: shell))
            stack.enter_context(
                patch.object(forge, "LocalFilesystem", lambda: filesystem)
            )
            ret = runner.invoke(
                main,
                ["config", "helm", "set", "velor-node", "--config", "temp2", "-y"],
                catch_exceptions=True,
            )
            shell.assert_commands(self)
            filesystem.assert_reads(self)
            filesystem.assert_writes(self)

    def testHelmSetConfigPreview(self) -> None:
        runner = CliRunner()
        shell = SpyShell(
            [
                FakeCommand(
                    "aws s3api get-object --bucket forge-wrapper-config --key "
                    "forge-wrapper-config.json temp1",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "aws s3api put-object --bucket forge-wrapper-config --key "
                    "forge-wrapper-config.json --body temp2",
                    RunResult(0, b""),
                ),
            ]
        )
        config_fixture_before = get_fixture_path(
            "forge-default-helm-values-before.fixture"
        )
        config_fixture_after = get_fixture_path(
            "forge-default-helm-values-after.fixture"
        )
        config_applied = json.loads(config_fixture_after.read_bytes().decode())[
            "default_helm_values"
        ]["velor-node"]
        config_fixture_preview = get_fixture_path(
            "forge-default-helm-values-preview.fixture"
        )
        filesystem = SpyFilesystem(
            {},
            {
                # read old config that has been written by s3 CLI
                "temp1": config_fixture_before.read_bytes(),
                # read the new *helm* config from disk
                "temp2": json.dumps(config_applied).encode(),
            },
        )
        with ExitStack() as stack:
            stack.enter_context(patch.object(forge, "LocalShell", lambda: shell))
            stack.enter_context(
                patch.object(forge, "LocalFilesystem", lambda: filesystem)
            )
            ret = runner.invoke(
                main,
                [
                    "--no-log-metadata",
                    "config",
                    "helm",
                    "set",
                    "velor-node",
                    "--config",
                    "temp2",
                    "-y",
                ],
                catch_exceptions=False,
            )
            shell.assert_commands(self)
            filesystem.assert_reads(self)
            filesystem.assert_writes(self)
            self.assertEqual(ret.exception, None)
            self.assertEqual(ret.exit_code, 0)
            assert ret.stdout_bytes.decode("utf-8").strip()
            self.assertEqual(
                ret.stdout_bytes.decode("utf-8").strip(),
                config_fixture_preview.read_bytes().decode("utf-8").strip(),
            )

    def testClusterDelete(self) -> None:
        runner = CliRunner()
        shell = SpyShell(
            [
                FakeCommand(
                    "aws s3api get-object --bucket forge-wrapper-config --key "
                    "forge-wrapper-config.json temp1",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "aws s3api put-object --bucket forge-wrapper-config --key "
                    "forge-wrapper-config.json --body temp2",
                    RunResult(0, b""),
                ),
            ]
        )
        clusters_before = {
            "enabled_clusters": ["banana"],
            "all_clusters": ["banana", "apple"],
        }
        clusters_after = {
            **clusters_before,
            "all_clusters": ["banana"],
        }
        filesystem = SpyFilesystem(
            {
                "temp2": json.dumps(clusters_after).encode(),
            },
            {
                "temp1": json.dumps(clusters_before).encode(),
            },
        )
        with ExitStack() as stack:
            stack.enter_context(patch.object(forge, "LocalShell", lambda: shell))
            stack.enter_context(
                patch.object(forge, "LocalFilesystem", lambda: filesystem)
            )
            runner.invoke(
                main,
                ["config", "cluster", "delete", "apple", "-y"],
                catch_exceptions=False,
            )
            shell.assert_commands(self)
            filesystem.assert_reads(self)
            filesystem.assert_writes(self)
