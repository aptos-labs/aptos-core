from test_framework.shell import SpyShell, FakeCommand, RunResult, Shell
from test_framework.filesystem import SpyFilesystem, Filesystem
from test_framework.kubernetes import SpyKubernetes, Kubernetes
from pangu_lib.testnet_commands.create_testnet import *
import unittest
import json
import asyncio


class testnet_tests_create_testnet(unittest.TestCase):
    def test_create_testnet(self) -> None:
        """Tests create_testnet on a CLI level"""

        #
        # Init fake vars
        pangu_node_configs_path: Optional[str] = "test_pangu_node_configs_path"
        layout_path: Optional[str] = "test_layout_path"
        framework_path: str = "test_head.mrb"
        num_of_validators: int = 10
        workspace: Optional[str] = "test_workspace"
        kubernetes: Kubernetes = SpyKubernetes()
        velor_cli_path: str = "test_velor_cli"
        dry_run: bool = False
        namespace: str = "test_namespace"

        #
        # Init expected filesystem reads/writes
        reads = {
            pangu_node_configs_path: open(
                "pangu_lib/template_testnet_files/pangu_node_config.yaml", "rb"
            ).read(),
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/layout.yaml": open(
                "pangu_lib/template_testnet_files/layout.yaml", "rb"
            ).read(),
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/waypoint.txt": b"waypoint",
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/genesis.blob": b"genesis",
        }
        writes = {
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}": b"",
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}": b"",
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/layout.yaml": open(
                "./pangu_lib/fixtures/layout_1.yaml", "rb"
            ).read(),
        }
        filesystem: Filesystem = SpyFilesystem(writes, reads)

        #
        # Init expected commands
        expected_commands = [
            FakeCommand(
                f"cp {framework_path} {workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/framework.mrb",
                RunResult(0, b"output"),
            ),
            FakeCommand(
                f"{velor_cli_path} genesis generate-genesis --local-repository-dir {workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace} --output-dir {workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}",
                RunResult(0, b"output"),
            ),
        ]
        shell: Shell = SpyShell(expected_commands)

        #
        # Run
        create_testnet_main(
            CreateArgs(
                pangu_node_configs_path=pangu_node_configs_path,
                num_of_validators=num_of_validators,
                layout_path=layout_path,
                workspace=workspace,
                framework_path=framework_path,
                velor_cli_path=velor_cli_path,
                dry_run=dry_run,
                name=namespace,
            ),
            SystemContext(shell, filesystem, kubernetes),
        )

        #
        # Assertions
        filesystem.assert_writes(self)
        filesystem.assert_reads(self)
        shell.assert_commands(self)

    def test_create_testnet_dry(self) -> None:
        """Tests create_testnet on a CLI level"""
        #
        # Init fake vars
        pangu_node_configs_path: Optional[str] = "test_pangu_node_configs_path"
        layout_path: Optional[str] = "test_layout_path"
        framework_path: str = "test_head.mrb"
        num_of_validators: int = 10
        workspace: Optional[str] = "test_workspace"
        kubernetes: Kubernetes = SpyKubernetes()
        velor_cli_path: str = "test_velor_cli"
        dry_run: bool = True
        namespace: str = "test_namespace"

        #
        # Init expected filesystem reads/writes
        reads = {
            pangu_node_configs_path: open(
                "pangu_lib/template_testnet_files/pangu_node_config.yaml", "rb"
            ).read(),
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/layout.yaml": open(
                "pangu_lib/template_testnet_files/layout.yaml", "rb"
            ).read(),
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/waypoint.txt": b"waypoint",
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/genesis.blob": b"genesis",
        }
        writes = {
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}": b"",
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}": b"",
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/dry_run": b"",
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/layout.yaml": open(
                "./pangu_lib/fixtures/layout_1.yaml", "rb"
            ).read(),
            f"{workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/dry_run/genesis_artifact_config_map.yaml": b"apiVersion: v1\nbinaryData:\n  genesis.blob: Z2VuZXNpcw==\ndata:\n  waypoint.txt: waypoint\nkind: ConfigMap\nmetadata:\n  name: genesis-artifiact-configmap-pangu\n",
        }
        filesystem: Filesystem = SpyFilesystem(writes, reads)

        #
        # Init expected commands
        expected_commands = [
            FakeCommand(
                f"cp {framework_path} {workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}/framework.mrb",
                RunResult(0, b"output"),
            ),
            FakeCommand(
                f"{velor_cli_path} genesis generate-genesis --local-repository-dir {workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace} --output-dir {workspace}/{util.PANGU_WORKSPACE_NAME}/{namespace}",
                RunResult(0, b"output"),
            ),
        ]
        shell: Shell = SpyShell(expected_commands)

        #
        # Run
        create_testnet_main(
            CreateArgs(
                pangu_node_configs_path=pangu_node_configs_path,
                num_of_validators=num_of_validators,
                layout_path=layout_path,
                workspace=workspace,
                framework_path=framework_path,
                velor_cli_path=velor_cli_path,
                dry_run=dry_run,
                name=namespace,
            ),
            SystemContext(shell, filesystem, kubernetes),
        )

        #
        # Assertions
        filesystem.assert_writes(self)
        filesystem.assert_reads(self)
        shell.assert_commands(self)

    def test_create_workspace(self) -> None:
        """Tests the create_workspace function"""
        #
        # Test mkdtemp
        filesystem: SpyFilesystem = SpyFilesystem(
            expected_writes={}, expected_reads={}, expected_unlinks=None
        )

        #
        # Create system_context
        system_context: SystemContext = SystemContext(
            SpyShell([]), filesystem, SpyKubernetes()
        )

        file_name: str = create_workspace(
            CreateArgs(
                pangu_node_configs_path="",
                num_of_validators=-1,
                layout_path="",
                workspace=None,
                framework_path="",
                velor_cli_path="",
                dry_run=False,
                name="default",
            ),
            system_context,
        )
        self.assertEqual(file_name, "temp_folder1")

        #
        # Test mkdir
        expected_writes2: Dict[str, bytes] = {
            "/tmp/pangu_artifacts": b"",
        }
        filesystem2: SpyFilesystem = SpyFilesystem(
            expected_writes=expected_writes2, expected_reads={}, expected_unlinks=None
        )
        #
        # Create system_context2
        system_context2: SystemContext = SystemContext(
            SpyShell([]), filesystem2, SpyKubernetes()
        )

        create_workspace(
            CreateArgs(
                pangu_node_configs_path="",
                num_of_validators=-1,
                layout_path="",
                workspace="/tmp",
                framework_path="",
                velor_cli_path="",
                dry_run=False,
                name="default",
            ),
            system_context=system_context2,
        )
        filesystem2.assert_writes(self)

    def test_temp_layout_creation(self) -> None:
        """Tests temp_layout_creation function"""
        #
        # Layout content to be used in the test
        layout_content = """
            root_key: "D04470F43AB6AEAA4EB616B72128881EEF77346F2075FFE68E14BA7DEBD8095E"
            users: []
            chain_id: 4
            allow_new_validators: false
            epoch_duration_secs: 7200
            is_test: true
            min_price_per_gas_unit: 1
            min_stake: 100000000000000
            min_voting_threshold: 100000000000000
            max_stake: 100000000000000000
            recurring_lockup_duration_secs: 86400
            required_proposer_stake: 1000000
            rewards_apy_percentage: 10
            voting_duration_secs: 43200
            voting_power_increase_limit: 20
        """

        #
        # Create the expected reads
        expected_reads: Dict[str, bytes] = {
            "/path/to/workspace/layout.yaml": layout_content.encode("utf-8"),
        }

        #
        # Create an instance of the SpyFilesystem
        filesystem: SpyFilesystem = SpyFilesystem(
            expected_writes={},
            expected_reads=expected_reads,
            expected_unlinks=None,
        )

        #
        # Set the layout path to None to test the default layout
        layout_path = None

        #
        # Create mock PanguNodeLayout instance
        blueprint1 = PanguNodeBlueprint(
            validator_storage_class_name="",
            vfn_storage_class_name="",
            validator_config_path="/path/to/node_config1.yaml",
            vfn_config_path="",
            create_vfns=False,
            stake_amount=100,
            vfn_image="",
            validator_image="",
            count=3,
            nodes_persistent_volume_claim_size="size",
            cpu="1",
            memory="1Gi",
        )

        blueprint2 = PanguNodeBlueprint(
            validator_storage_class_name="",
            vfn_storage_class_name="",
            validator_config_path="/path/to/node_config2.yaml",
            vfn_config_path="",
            create_vfns=False,
            stake_amount=200,
            vfn_image="",
            validator_image="",
            count=2,
            nodes_persistent_volume_claim_size="size",
            cpu="1",
            memory="1Gi",
        )

        #
        # Create a dictionary of blueprints
        blueprints: Dict[str, PanguNodeBlueprint] = {
            "blueprint1": blueprint1,
            "blueprint2": blueprint2,
        }
        mock_node_layout = PanguNodeLayout(blueprints=blueprints)

        #
        # Create system_context
        system_context: SystemContext = SystemContext(
            SpyShell([]), filesystem, SpyKubernetes()
        )

        #
        # Call the function you want to test
        result = temp_layout_creation(
            CreateArgs(
                pangu_node_configs_path="",
                num_of_validators=-1,
                layout_path=layout_path,
                workspace="/path/to/workspace",
                framework_path="",
                velor_cli_path="",
                dry_run=False,
                name="default",
            ),
            system_context=system_context,
            loaded_node_configs=mock_node_layout,
        )

        #
        # Ordering the write layout.yaml for testing
        dict1 = yaml.safe_load(filesystem.writes["/path/to/workspace/layout.yaml"])
        dict2 = yaml.safe_load(layout_content.encode("utf-8"))
        dict2["users"] = [
            "blueprint1-node-1",
            "blueprint1-node-2",
            "blueprint1-node-3",
            "blueprint2-node-1",
            "blueprint2-node-2",
        ]

        #
        # Assertions
        self.assertEqual(result, "/path/to/workspace/layout.yaml")
        self.assertEqual(
            json.dumps(dict1, sort_keys=True), json.dumps(dict2, sort_keys=True)
        )
        filesystem.assert_reads(self)
        filesystem.assert_unlinks(self)

    def test_parse_pangu_node_config(self) -> None:
        """Tests parse_pangu_node_config function"""
        #
        # Define the expected reads for the filesystem
        expected_reads = {
            "/path/to/pangu_node_configs.yaml": b"blueprints:\n \n nodebp:\n    nodes_persistent_volume_claim_size: size\n    validator_storage_class_name: standard\n    vfn_storage_class_name: standard\n    validator_config_path: /path/to/config.yaml\n    validator_image:  none\n    vfn_config_path:  /path/to/config2.yaml\n    vfn_image:  none\n    create_vfns: false\n    stake_amount: 1000\n    count: 10\n    cpu: '1'\n    memory: 1Gi\n"
        }

        #
        # Create an instance of the SpyFilesystem
        filesystem = SpyFilesystem(
            expected_writes={},
            expected_reads=expected_reads,
            expected_unlinks=None,
        )

        #
        # Create system_context
        system_context: SystemContext = SystemContext(
            SpyShell([]), filesystem, SpyKubernetes()
        )

        #
        # Call parse_pangu_node_config
        loaded_node_configs = parse_pangu_node_config(
            system_context,
            "/path/to/pangu_node_configs.yaml",
            10,
            True,
        )

        #
        # Create the expected PanguNodeBlueprint object
        expected_blueprint = PanguNodeBlueprint(
            validator_storage_class_name="standard",
            vfn_storage_class_name="standard",
            validator_config_path="/path/to/config.yaml",
            vfn_config_path="/path/to/config2.yaml",
            create_vfns=False,
            vfn_image="none",
            validator_image="none",
            stake_amount=1000,
            count=10,
            nodes_persistent_volume_claim_size="size",
            cpu="1",
            memory="1Gi",
        )

        #
        # Create the expected PanguNodeLayout object
        expected_node_layout = PanguNodeLayout(
            blueprints={"nodebp": expected_blueprint}
        )

        #
        # Assertions
        self.assertEqual(
            loaded_node_configs.blueprints, expected_node_layout.blueprints
        )
        filesystem.assert_reads(self)
        filesystem.assert_unlinks(self)

    def test_generate_genesis(self) -> None:
        """Tests generate_genesis function"""
        #
        # Load up the variables
        framework_path: str = "test_frm_path"
        workspace: str = "test_workspace"
        velor_cli_path: str = "test_velor"
        username: str = "blueprint1-node-1"
        user_dir: str = f"{workspace}/{username}"
        validator_host: str = f"{username}-validator:6180"
        fullnode_host: str = f"{username}-vfn:6182"
        cur_stake_amount: int = -1
        vfn_config_path: str = ""
        create_vfns: bool = False

        #
        # Create the expected commands
        expected_commands = [
            FakeCommand(
                f"{velor_cli_path} genesis generate-keys --output-dir {user_dir}",
                RunResult(0, b"output"),
            ),
            FakeCommand(
                f"{velor_cli_path} genesis set-validator-configuration --owner-public-identity-file {workspace}/blueprint1-node-1/public-keys.yaml --local-repository-dir {workspace} --username {username} --validator-host {validator_host} --full-node-host {fullnode_host} --stake-amount {cur_stake_amount}",
                RunResult(0, b"output"),
            ),
            FakeCommand(
                f"cp {framework_path} {workspace}/framework.mrb",
                RunResult(0, b"output"),
            ),
            FakeCommand(
                f"{velor_cli_path} genesis generate-genesis --local-repository-dir {workspace} --output-dir {workspace}",
                RunResult(0, b"output"),
            ),
        ]

        #
        # Create mock PanguNodeLayout instance
        blueprint1 = PanguNodeBlueprint(
            validator_storage_class_name="",
            vfn_storage_class_name="",
            validator_config_path="/path/to/node_config1.yaml",
            vfn_config_path=vfn_config_path,
            vfn_image="",
            validator_image="",
            create_vfns=create_vfns,
            stake_amount=cur_stake_amount,
            count=1,
            nodes_persistent_volume_claim_size="size",
            cpu="1",
            memory="1Gi",
        )

        #
        # Create a dictionary of blueprints
        blueprints: Dict[str, PanguNodeBlueprint] = {
            "blueprint1": blueprint1,
        }

        mock_node_layout = PanguNodeLayout(blueprints=blueprints)

        #
        # Run the async function
        shell: Shell = SpyShell(expected_commands)

        #
        # Create system_context
        system_context: SystemContext = SystemContext(
            shell, SpyFilesystem({}, {}), SpyKubernetes()
        )

        asyncio.run(
            generate_genesis(
                CreateArgs(
                    pangu_node_configs_path="pangu_node_configs_path",
                    num_of_validators=-1,
                    layout_path="layout_path",
                    workspace=workspace,
                    framework_path=framework_path,
                    velor_cli_path=velor_cli_path,
                    dry_run=True,
                    name="default",
                ),
                system_context,
                mock_node_layout,
            )
        )

        #
        # Assertions
        shell.assert_commands(self)

    def test_generate_keys_and_configuration(self) -> None:
        """Tests generate_keys_and_configuration function"""
        #
        # Init fake vars
        username: str = "test_username"
        user_dir: str = "test_userdir"
        validator_host: str = "test_val_host"
        fullnode_host: str = "test_ful_host"
        cur_stake_amount: int = -1
        workspace: str = "test_workspace"
        velor_cli_path: str = "test_velor"
        vfn_config_path: str = ""
        create_vfns: bool = False

        #
        # Create the expected fake commands
        fake_commands = [
            FakeCommand(
                " ".join(
                    [
                        velor_cli_path,
                        "genesis",
                        "generate-keys",
                        "--output-dir",
                        user_dir,
                    ]
                ),
                RunResult(0, b""),
            ),
            FakeCommand(
                " ".join(
                    [
                        velor_cli_path,
                        "genesis",
                        "set-validator-configuration",
                        "--owner-public-identity-file",
                        user_dir + "/public-keys.yaml",
                        "--local-repository-dir",
                        workspace,
                        "--username",
                        username,
                        "--validator-host",
                        validator_host,
                        "--full-node-host",
                        fullnode_host,
                        "--stake-amount",
                        str(cur_stake_amount),
                    ]
                ),
                RunResult(0, b""),
            ),
        ]

        #
        # Run the async function
        shell: Shell = SpyShell(fake_commands)
        system_context: SystemContext = SystemContext(
            shell, SpyFilesystem({}, {}), SpyKubernetes()
        )
        asyncio.run(
            generate_keys_and_configuration(
                GenesisNodeInformation(
                    util.DEFAULT_IMAGE,
                    util.DEFAULT_IMAGE,
                    "storage",
                    "storage",
                    username,
                    user_dir,
                    validator_host,
                    fullnode_host,
                    cur_stake_amount,
                    "node_config_path",
                    vfn_config_path,
                    "persistent_storage_size",
                    create_vfns,
                    "1",
                    "1Gi",
                ),
                CreateArgs(
                    pangu_node_configs_path="",
                    num_of_validators=-1,
                    layout_path="",
                    workspace=workspace,
                    framework_path="",
                    velor_cli_path=velor_cli_path,
                    dry_run=True,
                    name="default",
                ),
                system_context,
            )
        )

        #
        # Assertions
        shell.assert_commands(self)

    def test_get_layout_node_count(self) -> None:
        """Tests get_layout_node_count function"""
        #
        # Define the expected reads for the filesystem
        expected_node_count: int = 5
        expected_reads = {
            "/path/to/pangu_node_configs.yaml": b"blueprints:\n \n nodebp:\n    nodes_persistent_volume_claim_size: size\n    validator_storage_class_name: standard\n    vfn_storage_class_name: standard\n    validator_config_path: /path/to/config.yaml\n    validator_image:  none\n    vfn_config_path:  /path/to/config2.yaml\n    vfn_image:  none\n    create_vfns: false\n    stake_amount: 1000\n    count: 10\n    cpu: '1'\n    memory: 1Gi\n"
        }

        #
        # Create an instance of the SpyFilesystem
        filesystem = SpyFilesystem(
            expected_writes={},
            expected_reads=expected_reads,
            expected_unlinks=None,
        )

        system_context: SystemContext = SystemContext(
            SpyShell([]), filesystem, SpyKubernetes()
        )
        #
        # Call parse_pangu_node_config
        loaded_node_configs = parse_pangu_node_config(
            system_context,
            "/path/to/pangu_node_configs.yaml",
            expected_node_count,
            True,
        )

        #
        # Call layout node count function
        node_count: int = get_layout_node_count(loaded_node_configs)

        #
        # Assertion
        self.assertEqual(node_count, expected_node_count)

    async def test_create_stateful_set_validator(self) -> None:
        """Tests create_validator_stateful_set function"""
        #
        # Init fake vars
        username: str = "test_username"
        workspace: str = "test_workspace"
        kubernetes: SpyKubernetes = SpyKubernetes()
        dry_run: bool = True
        namespace: str = "testing_namespace"

        #
        # Init expected filesystem reads/writes
        writes = {
            "test_workspace/dry_run/test_username/test_username-validator-statefulset.yaml": open(
                "./pangu_lib/fixtures/stateful_validator_1.yaml", "rb"
            ).read(),
            "test_workspace/dry_run/test_username/test_username-validator-service.yaml": open(
                "./pangu_lib/fixtures/service_validator_1.yaml", "rb"
            ).read(),
            "test_workspace/dry_run/test_username/test_username-validator-pvc.yaml": open(
                "./pangu_lib/fixtures/pvc_validator_1.yaml", "rb"
            ).read(),
        }
        filesystem: SpyFilesystem = SpyFilesystem(writes, {})

        #
        # Run
        loop = asyncio.get_event_loop()
        system_context: SystemContext = SystemContext(
            SpyShell([]), filesystem, kubernetes
        )
        result = loop.run_until_complete(
            create_node_stateful_sets(
                CreateArgs(
                    pangu_node_configs_path="pangu_node_configs_path",
                    num_of_validators=-1,
                    layout_path="layout_path",
                    workspace=workspace,
                    framework_path="framework_path",
                    velor_cli_path="velor_cli_path",
                    dry_run=dry_run,
                    name=namespace,
                ),
                system_context,
                util.NodeType.VALIDATOR,
                username,
                util.DEFAULT_IMAGE,
            )
        )

        #
        # Assertions
        self.maxDiff = None
        filesystem.assert_writes(self)
        self.assertEqual(result, None)

    async def test_create_genesis_secrets_and_configmaps_no_vfn(self) -> None:
        """Tests create_validator_genesis_secrets_and_configmaps function"""
        #
        # Init expected filesystem reads/writes
        writes = {
            "test_workspace/dry_run/test_username/validator_config_config_map.yaml": b"apiVersion: v1\ndata:\n  validator.yaml: ''\nkind: ConfigMap\nmetadata:\n  name: test_username-validator-configmap\n",
            "test_workspace/dry_run/test_username/identity_secrets.yaml": b"apiVersion: v1\nkind: Secret\nmetadata:\n  name: identity-secrets-test_username\nstringData:\n  validator-full-node-identity.yaml: ''\n  validator-identity.yaml: ''\n",
        }
        reads = {
            "test_node_config_path": b"",
            "test_userdir/validator-identity.yaml": b"",
            "test_userdir/validator-full-node-identity.yaml": b"",
        }
        filesystem: SpyFilesystem = SpyFilesystem(writes, reads)

        #
        # Init fake vars
        username: str = "test_username"
        workspace: str = "test_workspace"
        user_dir: str = "test_userdir"
        kubernetes: SpyKubernetes = SpyKubernetes()
        validator_config_path: str = "test_node_config_path"
        dry_run: bool = True
        namespace: str = "test_namespace"
        vfn_config_path: str = ""
        create_vfns: bool = False

        #
        # Run
        loop = asyncio.get_event_loop()
        system_context: SystemContext = SystemContext(
            SpyShell([]), filesystem, kubernetes
        )
        result = loop.run_until_complete(
            create_genesis_secrets_and_configmaps(
                GenesisNodeInformation(
                    util.DEFAULT_IMAGE,
                    util.DEFAULT_IMAGE,
                    "storage",
                    "storage",
                    username,
                    user_dir,
                    "validator_host",
                    "fullnode_host",
                    1000,
                    validator_config_path,
                    vfn_config_path,
                    "persistent_storage_size",
                    create_vfns,
                    "1",
                    "1Gi",
                ),
                CreateArgs(
                    pangu_node_configs_path="pangu_node_configs_path",
                    num_of_validators=-1,
                    layout_path="layout_path",
                    workspace=workspace,
                    framework_path="framework_path",
                    velor_cli_path="velor_cli_path",
                    dry_run=dry_run,
                    name=namespace,
                ),
                system_context,
            )
        )

        #
        # Assertions
        self.assertEqual(result, None)
        filesystem.assert_writes(self)
        filesystem.assert_reads(self)

    async def test_create_stateful_set_vfn(self) -> None:
        """Tests create_validator_stateful_set function"""
        #
        # Init fake vars
        username: str = "test_username"
        workspace: str = "test_workspace"
        kubernetes: SpyKubernetes = SpyKubernetes()
        dry_run: bool = True
        namespace: str = "testing_namespace"

        #
        # Init expected filesystem reads/writes
        writes = {
            "test_workspace/dry_run/test_username/test_username-vfn-statefulset.yaml": open(
                "./pangu_lib/fixtures/stateful_vfn_1.yaml", "rb"
            ).read(),
            "test_workspace/dry_run/test_username/test_username-vfn-service.yaml": open(
                "./pangu_lib/fixtures/service_vfn_1.yaml", "rb"
            ).read(),
            "test_workspace/dry_run/test_username/test_username-vfn-pvc.yaml": open(
                "./pangu_lib/fixtures/pvc_vfn_1.yaml", "rb"
            ).read(),
        }
        filesystem: SpyFilesystem = SpyFilesystem(writes, {})

        #
        # Run
        loop = asyncio.get_event_loop()
        system_context: SystemContext = SystemContext(
            SpyShell([]), filesystem, kubernetes
        )
        result = loop.run_until_complete(
            create_node_stateful_sets(
                CreateArgs(
                    pangu_node_configs_path="pangu_node_configs_path",
                    num_of_validators=-1,
                    layout_path="layout_path",
                    workspace=workspace,
                    framework_path="framework_path",
                    velor_cli_path="velor_cli_path",
                    dry_run=dry_run,
                    name=namespace,
                ),
                system_context,
                util.NodeType.VFN,
                username,
                util.DEFAULT_IMAGE,
            )
        )
        self.maxDiff = None
        #
        # Assertions
        filesystem.assert_writes(self)
        self.assertEqual(result, None)

    async def test_create_genesis_secrets_and_configmaps_with_vfn(self) -> None:
        """Tests create_validator_genesis_secrets_and_configmaps function"""
        #
        # Init expected filesystem reads/writes
        writes = {
            "test_workspace/dry_run/test_username/validator_config_config_map.yaml": b"apiVersion: v1\ndata:\n  validator.yaml: ''\nkind: ConfigMap\nmetadata:\n  name: test_username-validator-configmap\n",
            "test_workspace/dry_run/test_username/identity_secrets.yaml": b"apiVersion: v1\nkind: Secret\nmetadata:\n  name: identity-secrets-test_username\nstringData:\n  validator-full-node-identity.yaml: ''\n  validator-identity.yaml: ''\n",
        }
        reads = {
            "test_workspace/test_username/vfn.yaml": open(
                "./pangu_lib/fixtures/vfn_1.yaml", "rb"
            ).read(),
            f"{util.TEMPLATE_DIRECTORY}/vfn.yaml": open(
                "./pangu_lib/template_testnet_files/vfn.yaml", "rb"
            ).read(),
            "test_node_config_path": b"",
            "test_userdir/validator-identity.yaml": b"",
            "test_userdir/validator-full-node-identity.yaml": b"",
        }
        filesystem: SpyFilesystem = SpyFilesystem(writes, reads)

        #
        # Init fake vars
        username: str = "test_username"
        workspace: str = "test_workspace"
        user_dir: str = "test_userdir"
        kubernetes: SpyKubernetes = SpyKubernetes()
        validator_config_path: str = "test_node_config_path"
        dry_run: bool = True
        namespace: str = "test_namespace"
        vfn_config_path: str = "test_vfn_config_path"
        create_vfns: bool = True

        #
        # Run
        loop = asyncio.get_event_loop()
        system_context: SystemContext = SystemContext(
            SpyShell([]), filesystem, kubernetes
        )
        result = loop.run_until_complete(
            create_genesis_secrets_and_configmaps(
                GenesisNodeInformation(
                    util.DEFAULT_IMAGE,
                    util.DEFAULT_IMAGE,
                    "storage",
                    "storage",
                    username,
                    user_dir,
                    "validator_host",
                    "fullnode_host",
                    1000,
                    validator_config_path,
                    vfn_config_path,
                    "persistent_storage_size",
                    create_vfns,
                    "1",
                    "1Gi",
                ),
                CreateArgs(
                    pangu_node_configs_path="pangu_node_configs_path",
                    num_of_validators=-1,
                    layout_path="layout_path",
                    workspace=workspace,
                    framework_path="framework_path",
                    velor_cli_path="velor_cli_path",
                    dry_run=dry_run,
                    name=namespace,
                ),
                system_context,
            )
        )

        #
        # Assertions
        self.assertEqual(result, None)
        filesystem.assert_writes(self)
        filesystem.assert_reads(self)
