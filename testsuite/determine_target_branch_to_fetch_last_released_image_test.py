import unittest
from unittest.mock import patch, MagicMock
from datetime import datetime
import sys

from determine_target_branch_to_fetch_last_released_image import (
    determine_target_branch,
    get_latest_branch_for_previous_major,
    get_all_release_branches,
    get_all_release_branches_with_times,
)

class TestDetermineTargetBranch(unittest.TestCase):

    @patch('determine_target_branch_to_fetch_last_released_image.git')
    def test_determine_target_branch_to_fetch_last_released_image_main(self, mock_git):
        """Test case for the 'main' branch to get the latest release branch."""
        mock_git.get_remote_branches_matching_pattern.return_value = [
            'aptos-release-v1.19', 'aptos-release-v1.18'
        ]
        mock_git.get_branch_creation_time.side_effect = lambda branch: datetime(2024, 9, 1) if branch == 'aptos-release-v1.19' else datetime(2024, 8, 1)

        result = determine_target_branch('main')

        self.assertEqual(result, 'aptos-release-v1.19')

    @patch('determine_target_branch_to_fetch_last_released_image.git')
    def test_determine_target_branch_to_fetch_last_released_image_aptos_release(self, mock_git):
        """Test case for determining target branch when the base branch is an aptos-release-vX.Y branch."""
        mock_git.get_remote_branches_matching_pattern.return_value = [
            'aptos-release-v1.20', 'aptos-release-v1.19', 'aptos-release-v1.18', 'aptos-release-v1.17'
        ]

        # Mock the branch creation times
        mock_git.get_branch_creation_time.side_effect = lambda branch: {
            'aptos-release-v1.20': '2023-09-20 12:00:00 +0000',
            'aptos-release-v1.19': '2023-06-15 12:00:00 +0000',
            'aptos-release-v1.18': '2023-03-10 12:00:00 +0000',
            'aptos-release-v1.17': '2023-01-05 12:00:00 +0000',
        }[branch]

        result = determine_target_branch('aptos-release-v1.19')
        self.assertEqual(result, 'aptos-release-v1.18')

        mock_git.get_remote_branches_matching_pattern.return_value = [
            'aptos-release-v0.1', 'aptos-release-v0.11'
        ]
        mock_git.get_branch_creation_time.side_effect = lambda branch: {
            'aptos-release-v1.18': '2023-03-10 12:00:00 +0000',
            'aptos-release-v1.0':  '2023-01-01 12:00:00 +0000',
            'aptos-release-v0.11': '2022-07-01 12:00:00 +0000',
            'aptos-release-v0.1':  '2022-06-01 12:00:00 +0000',
        }[branch]

        result = determine_target_branch('aptos-release-v1.0')
        self.assertEqual(result, 'aptos-release-v0.11')

    @patch('determine_target_branch_to_fetch_last_released_image.git')
    def test_determine_target_branch_to_fetch_last_released_image_personal_branch(self, mock_git):
        """Test case for determining target branch when base branch is a personal branch."""
        mock_git.get_remote_branches_matching_pattern.return_value = [
            'aptos-release-v1.19', 'aptos-release-v1.18'
        ]
        mock_git.get_branch_creation_time.side_effect = lambda branch: datetime(2024, 9, 1) if branch == 'aptos-release-v1.19' else datetime(2024, 8, 1)

        mock_git.get_branch_creation_time.return_value = datetime(2024, 8, 15)

        result = determine_target_branch('personal-branch')
        self.assertEqual(result, 'aptos-release-v1.18')

    @patch('determine_target_branch_to_fetch_last_released_image.git')
    def test_get_latest_branch_for_previous_major(self, mock_git):
        """Test case for fetching the latest branch of a previous major version."""
        mock_git.get_remote_branches_matching_pattern.return_value = [
            'aptos-release-v0.2', 'aptos-release-v0.1'
        ]

        result = get_latest_branch_for_previous_major(1)
        self.assertEqual(result, 'aptos-release-v0.2')

    @patch('determine_target_branch_to_fetch_last_released_image.git')
    def test_get_all_release_branches(self, mock_git):
        """Test case for fetching all release branches."""
        mock_git.get_remote_branches_matching_pattern.return_value = [
            'aptos-release-v1.19', 'aptos-release-v1.18', 'aptos-release-v1.17'
        ]
        
        result = get_all_release_branches()
        self.assertEqual(result, ['aptos-release-v1.17', 'aptos-release-v1.18', 'aptos-release-v1.19'])

    @patch('determine_target_branch_to_fetch_last_released_image.git')
    def test_get_all_release_branches_with_times(self, mock_git):
        """Test case for fetching all release branches with their creation times."""
        mock_git.get_remote_branches_matching_pattern.return_value = [
            'aptos-release-v1.19', 'aptos-release-v1.18'
        ]
        mock_git.get_branch_creation_time.side_effect = lambda branch: datetime(2024, 9, 1) if branch == 'aptos-release-v1.19' else datetime(2024, 8, 1)

        result = get_all_release_branches_with_times()
        expected = [('aptos-release-v1.18', datetime(2024, 8, 1)), ('aptos-release-v1.19', datetime(2024, 9, 1))]
        self.assertEqual(result[:2], expected)

if __name__ == "__main__":
    unittest.main()
