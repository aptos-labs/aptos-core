# This script installs the necessary dependencies to build in Aptos.

$ErrorActionPreference = 'Stop'

Set-Location (Split-Path -Parent $MyInvocation.MyCommand.Path) | Out-Null; Set-Location '..' -ErrorAction Stop

$global:os = $null
$global:architecture = $null
$global:msvcpath = $null

function verify_architecture {  # Checks whether the Windows machine is 32-bit or 64-bit
	$result = Get-WmiObject -Class Win32_Processor | Select-Object AddressWidth |ConvertTo-Json -Compress
	if ($result.Contains("64")) {
		$global:architecture = "64"
		Write-Host "64-bit system detected"
		}
	else {
		$global:architecture = "86"
		Write-Host "32-bit system detected"
		}
}

function check_os {
	$osName = (Get-WMIObject win32_operatingsystem).name
	if ($osName.Contains("Windows 10")) {
		Write-Host "Supported Windows OS detected"
		$global:os = "Windows 10"
	}
	elseif ($osName.Contains("Windows 11")) {
		Write-Host "Supported Windows OS detected"
		$global:os = "Windows 11"
	}
	elseif ($osName.Contains("Windows Server 2022")) {
		Write-Host "Supported Windows OS"
		$global:os = "Windows Server 2022 detected"
	}
	else {
		Write-Host "Unsupported Windows OS detected. Stopping script..."
		Exit
	}
}

function check_package {
  param(
    [string]$package
  )
  if ($(winget list --name $package --accept-source-agreements) -match 'No installed package found matching input criteria.') {
    Write-Host "Installing $package..."
    return $true
  }   
  else {
    Write-Host "$package is already installed."
  }
}

function install_msvc_build_tools {  # Installs C++ build tools, CMake, and Windows 10/11 SDK
  $result = check_package "Visual Studio Build Tools"
  if ($result) {
	select_msvc_variant
    set_msvc_env_variables
	}
}

function select_msvc_variant {  # Decides between the Windows 10 SDK and Windows 11 SDK based on your OS
	if ($global:os -eq "Windows 11") {
		winget install Microsoft.VisualStudio.2022.BuildTools --accept-source-agreements --silent --override "--wait --quiet --add ProductLang En-us --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.22621 --add Microsoft.VisualStudio.Component.VC.CMake.Project --includeRecommended"
		}
	else {
		winget install Microsoft.VisualStudio.2022.BuildTools --accept-source-agreements --silent --override "--wait --quiet --add ProductLang En-us --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows10SDK.20348 --add Microsoft.VisualStudio.Component.VC.CMake.Project --includeRecommended"
		}
}

function get_msvc_install_path {
	$msvcpath = Get-CimInstance -ClassName "MSFT_VSInstance" | Select-Object -ExpandProperty InstallLocation
	return $msvcpath
}

function get_msvc_version {  # Finds the MSVC version number and creates a valid filepath to add as an environment variable
    $global:msvcpath = get_msvc_install_path
	$pathpattern = "$msvcpath\VC\Tools\MSVC\*\bin\Hostx64\x64\link.exe"

    # Get the file path that matches the pattern
    $filepath = Get-ChildItem -Path $pathpattern -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1

    if ($filepath) {
        # Extract the version number from the file path using regex
        $msvcversion = $filepath.FullName -replace ".*MSVC\\(\d+\.\d+\.\d+)\\.*", '$1'
		return $msvcversion
    } else {
        Write-Warning "MSVC not found: $pathpattern"
        return $null
    }
}

function set_msvc_env_variables {  # Sets the environment variables based on the architecture and MSVC version
    $msvcversion = get_msvc_version
	[Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$global:msvcpath\VC\Tools\MSVC\$msvcversion\bin\Hostx$global:architecture\x$global:architecture\link.exe", "User")
	[Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$global:msvcpath\VC\Tools\MSVC\$msvcversion\bin\Hostx$global:architecture\x$global:architecture\cl.exe", "User")
	Write-Host "Environment variables set"
}

function install_rustup {
  $result = check_package "Rustup"
  if ($result) {
	winget install Rustlang.Rustup --silent
	Exit
	}
  Write-Host "Configuring Rustup..."
  rustup update
  rustup component add rustfmt
  rustup component add clippy
  rustup toolchain install nightly
  rustup component add rustfmt --toolchain nightly
}

function install_llvm {
  $result = check_package "LLVM"
  if ($result) {
	winget install LLVM.LLVM --silent
	}
}

function install_openssl {
  $result = check_package "OpenSSL"
  if ($result) {
	winget install ShiningLight.OpenSSL --silent
	}
}

function install_nodejs {
  $result = check_package "Node.js"
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
  $result = check_package "PostgreSQL"
  if ($result) {
	winget install PostgreSQL.PostgreSQL --silent
	}
}

function existing_package {
	Write-Host "This package is already installed."
}

check_os
verify_architecture
install_msvc_build_tools
install_llvm
install_openssl
install_nodejs
install_pnpm
install_postgresql
install_rustup
Write-Host "Finished..."
