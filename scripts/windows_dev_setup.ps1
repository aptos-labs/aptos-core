# This script installs the necessary dependencies to build in Aptos.

$ErrorActionPreference = 'Stop'

Set-Location (Split-Path -Parent $MyInvocation.MyCommand.Path) | Out-Null; Set-Location '..' -ErrorAction Stop

$global:os = $null
$global:architecture = $null

function check_os {
	$osName = (Get-WMIObject win32_operatingsystem).name
	if ($osName.Contains("Windows 10")) {
		Write-Host "Supported Windows OS"
		$global:os = "Windows 10"
	}
	elseif ($osName.Contains("Windows 11")) {
		Write-Host "Supported Windows OS"
		$global:os = "Windows 11"
	}
	elseif ($osName.Contains("Windows Server 2022")) {
		Write-Host "Supported Windows OS"
		$global:os = "Windows Server 2022"
	}
	else {
		Write-Host "Unsupported Windows OS"
		Exit
	}
}

function check_package {
  param(
    [string]$package
  )
  if ($(winget list --name $package) -match 'No installed package found matching input criteria.') {
    Write-Host "Installing $package..."
    return $true
  }   
  else {
    Write-Host "$package is already installed..."
  }
}

function install_msvc_build_tools {  # Installs C++ build tools, CMake, and Windows 10/11 SDK
  $result = check_package "buildtools"
  if ($result) {
	install_variant
	}
}

function install_variant {  # Decides between the Windows 10 SDK and Windows 11 SDK based on your OS
	if ($global:os -eq "Windows 11") {
		winget install Microsoft.VisualStudio.2022.BuildTools --accept-source-agreements --silent --override "--wait --quiet --add ProductLang En-us --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.22621 --add Microsoft.VisualStudio.Component.VC.CMake.Project --includeRecommended"
		}
	else {
		winget install Microsoft.VisualStudio.2022.BuildTools --accept-source-agreements --silent --override "--wait --quiet --add ProductLang En-us --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows10SDK.20348 --add Microsoft.VisualStudio.Component.VC.CMake.Project --includeRecommended"
		}
}

function get_msvc_version {  # Finds the MSVC version number and creates a valid filepath to add as an environment variable
    $pathpattern = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\*\bin\Hostx64\x64\link.exe"

    # Get the file path that matches the pattern
    $filepath = Get-ChildItem -Path $pathpattern -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1

    if ($filepath) {
        # Extract the version number from the file path using regex
        $global:msvcversion = $filepath.FullName -replace ".*MSVC\\(\d+\.\d+\.\d+)\\.*", '$1'
    } else {
        Write-Warning "MSVC not found: $pathpattern"
        return $null
    }
}

function verify_architecture {  # Checks whether the Windows machine is 32-bit or 64-bit
	$result = Get-WmiObject -Class Win32_Processor | Select-Object AddressWidth
	if ($result.Contains("64")) {
		$global:architecture = "64"
		}
	else {
		$global:architecture = "86"
		}
}

function set_msvc_env_variables {  # Sets the environment variables based on the architecture and MSVC version
	[Environment]::SetEnvironmentVariable("PATH", "$env:PATH;C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\$global:msvcversion\bin\Hostx$global:architecture\x$global:architecture\link.exe", "User")
	[Environment]::SetEnvironmentVariable("PATH", "$env:PATH;C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\$global:msvcversion\bin\Hostx$global:architecture\x$global:architecture\cl.exe", "User")
	Write-Host "Environment variables set"
}

function install_rustup {
  $result = check_package "rustup"
  if ($result) {
	winget install Rustlang.Rustup --silent
	Exit
	Powershell
	}
  rustup update
  rustup component add rustfmt
  rustup component add clippy
  rustup toolchain install nightly
  rustup component add rustfmt --toolchain nightly
}

function install_llvm {
  $result = check_package "llvm"
  if ($result) {
	winget install LLVM.LLVM --silent
	}
}

function install_openssl {
  $result = check_package "openssl"
  if ($result) {
	winget install ShiningLight.OpenSSL --silent
	}
}

function install_nodejs {
  $result = check_package "nodejs"
  if ($result) {
	winget install OpenJS.NodeJS --silent
	}
}

function install_pnpm {
  $result = check_package "pnpm"
  if ($result) {
	winget install pnpm.pnpm --silent
	}
}

function install_postgresql {
  $result = check_package "postgresql"
  if ($result) {
	winget install PostgreSQL.PostgreSQL --silent
	}
}

function existing_package {
	Write-Host "This package is already installed..."
}


check_os
install_msvc_build_tools
verify_architecture
set_msvc_env_variables
install_llvm
install_openssl
install_nodejs
install_pnpm
install_postgresql
install_rustup
Write-Host "Finished..."