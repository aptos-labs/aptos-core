# This script installs the necessary dependencies to build in Velor.

param (
    [switch]$t,
    [switch]$y
)

$ErrorActionPreference = 'Stop'

Set-PSDebug -Trace 1

Set-Location (Split-Path -Parent $MyInvocation.MyCommand.Path) | Out-Null; Set-Location '..' -ErrorAction Stop

$global:user_selection = $null
$global:os = $null
$global:architecture = $null
$global:msvcpath = $null
$global:cvc5_version = "0.0.8"
$global:grcov_version = "GRCOV_VERSION="
$global:protoc_version = "PROTOC_VERSION="
$global:dotnet_version = "DOTNET_VERSION="
$global:z3_version = "Z3_VERSION="
$global:boogie_version = "BOOGIE_VERSION="


function welcome_message {
    $message = "`nWelcome to Velor!
    `nThis script will download and install the necessary dependencies for Velor Core based on your selection:
      * Install Velor build tools: t
      * Install Move Prover tools: y`n
      Selection"

    return $message
}

function build_tools_message {
    $message = "`nYou selected option 't'.
    `nThis script will download and install the following dependencies needed to build Velor Core if not found on your system:
    * Rust (and necessary components)
      * rust-fmt
      * clippy
      * cargo-sort
      * cargo-nextest
    * MSVC Build Tools - Desktop development with C++ (and necessary components)
      * MSVC C++ build tools
      * Windows 10/11 SDK
    * Protoc (and necessary components)
      * protoc-gen-prost
      * protoc-gen-prost-serde
      * protoc-gen-prost-crate
    * Python (and necessary components)
      * pip
    * LLVM
    * CMake
    * OpenSSL
    * NodeJS
    * NPM
    * PostgreSQL
    * Grcov"

    return $message
}

function move_prover_message {
    $message = "`nYou selected option 'y'.
    `nThis script will download and install the following dependencies needed to use the Move Prover if not found on your system:
    * Dotnet
    * Z3
    * Boogie
    * CVC5"
    return $message
}

function update_versions {
  try {
    # URL of the Unix script
    $url = "https://raw.githubusercontent.com/velor-chain/velor-core/main/scripts/dev_setup.sh"

    # Retrieve the content of the file and store it in a variable
    $content = (Invoke-WebRequest -UseBasicParsing -Uri $url | Select-Object -ExpandProperty Content -First 50) -join "`n"

    $packages = @($global:grcov_version, $global:protoc_version, $global:dotnet_version, $global:z3_version, $global:boogie_version)

    foreach ($package in $packages) {
      $index = $content.IndexOf($package)

      # If the search pattern was found, extract the matching line of text
      if ($index -ge 0) {
          # Find the end of the line by searching for the next newline character
          $end_index = $content.IndexOf("`n", $index)

          # Extract the line of text that matches the search pattern
          if ($end_index -ge 0) {
            $matching_line = $content.Substring($index, $end_index - $index)
          } else {
            $matching_line = $content.Substring($index)
          }
          # Extract the version number from the line of text
          $matching_text = $matching_line.Split('=')[1].Trim()

          $package_name = $package.TrimEnd('=')
          # Update global variable with the extract version
          Set-Variable -Name ($package_name) -Value $matching_text -Scope Global -Force

          if ($matching_text -notmatch '\d+\.\d+(\.\d+)?') {
            Write-Error "$package_name cannot be read due to a formatting problem in the source file."
          }
        }
      else {
        Write-Error "Updated $package_name not found."
      }
    }
   }
  catch {
    Write-Error "Unable to check for updated version numbers for some dependencies due to an error: $($_.Exception.Message)"
    Write-Host "Installation will continue with the current versions..."
    $global:grcov_version = "0.8.2"
    $global:protoc_version = "21.4"
    $global:dotnet_version = "6.0"
    $global:z3_version = "4.11.2"
    $global:boogie_version = "3.0.1"
  }
}

function verify_architecture {  # Checks whether the Windows machine is 32-bit or 64-bit
	$result = Get-WmiObject -Class Win32_Processor | Select-Object AddressWidth |ConvertTo-Json -Compress
	if ($result.Contains("64")) {
		$global:architecture = "64"
		}
	else {
		$global:architecture = "86"
		}
}

function check_os {
	$osName = (Get-WMIObject win32_operatingsystem).name
	if ($osName.Contains("Windows 10")) {
		$global:os = "Windows 10"
	}
	elseif ($osName.Contains("Windows 11")) {
		$global:os = "Windows 11"
	}
	elseif ($osName.Contains("Windows Server 20")) {
		$global:os = "Windows Server 20XX"
	}
	else {
		Write-Host "Unsupported Windows OS detected. Stopping script..."
		Exit
	}
}

function install_winget {
  $xaml_url = "https://globalcdn.nuget.org/packages/microsoft.ui.xaml.2.8.6.nupkg"
  $xaml_downloadpath = "Microsoft.UI.Xaml.2.8.6.nupkg.zip"
  $xaml_filepath = "Microsoft.UI.Xaml.2.8.6.nupkg\tools\AppX\x64\Release\Microsoft.UI.Xaml.2.8.appx"

  $vclib_url = "https://aka.ms/Microsoft.VCLibs.x$global:architecture.14.00.Desktop.appx"
  $vclib_downloadpath = "Microsoft.VCLibs.x$global:architecture.14.00.Desktop.appx"

  $installer_url = "https://github.com/microsoft/winget-cli/releases/download/v1.7.11132/Microsoft.DesktopAppInstaller_8wekyb3d8bbwe.msixbundle"
  $installer_downloadpath = "msftwinget.msixbundle"

  $license_url = "https://github.com/microsoft/winget-cli/releases/download/v1.7.11132/ccfd1d114c9641fc8491f3c7c179829e_License1.xml"
  $license_downloadpath = "license.xml"

  # Download and extract XAML (dependency)
  Safe-Download-File $xaml_url -Destination $xaml_downloadpath
  Expand-Archive $xaml_downloadpath -ErrorAction SilentlyContinue

  # Download and install VCLibs and XAML (dependencies)
  Safe-Download-File -Source $vclib_url -Destination $vclib_downloadpath
  Add-AppxPackage $vclib_downloadpath
  Add-AppxPackage $xaml_filepath

  # Download and install WinGet
  Safe-Download-File -Source $installer_url -Destination $installer_downloadpath
  Safe-Download-File -Source $license_url -Destination $license_downloadpath
  Add-AppxProvisionedPackage -Online -PackagePath $installer_downloadpath -LicensePath $license_downloadpath

  # Cleanup
  Remove-Item $xaml_filepath
  Remove-Item $vclib_downloadpath
  Remove-Item $installer_downloadpath
  Remove-Item $license_downloadpath

  # Add WinGet directory to user PATH environment variable
  [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$env:LOCALAPPDATA\Microsoft\WindowsApps", "User")

  # Reload the PATH environment variables for this session
  $env:Path = [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "Machine")
  Write-Host "WinGet has been installed. We recommend stopping the script and restarting your system to ensure WinGet has been set up correctly before continuing."
}

function check_for_winget {
  if (Get-Command winget -ErrorAction SilentlyContinue) {
    return
  }
  elseif (Test-Path "$env:LOCALAPPDATA\Microsoft\WindowsApps\winget.exe") {
    [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$env:LOCALAPPDATA\Microsoft\WindowsApps", "User")
    # Reload the PATH environment variables for this session
    $env:Path = [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "Machine")
  }
  else {
    Write-Host "Installing WinGet before continuing with the script..."
    install_winget
  }
}

function check_package { # Checks for packages installed with winget or typical installers
  param(
    [string]$package
  )
  if ((winget list --name $package) -match 'No installed package found matching input criteria.') {
    Write-Host "Installing $package..."
    return $true
  }
  elseif ((winget upgrade | Out-String).Contains($package)) {
    Write-Host "$package is already installed, but an update is available."
    return $false
  }
  else {
    Write-Host "$package is already installed and up-to-date."
  }
}

function install_msvc_build_tools {  # Installs C++ build tools, CMake, and Windows 10/11 SDK
  $result = check_package "Visual Studio Build Tools"
  if ($result) {
    select_msvc_variant
    set_msvc_env_variables
	}
  else {
    Write-Host "Installing update..."
    winget upgrade --id Microsoft.VisualStudio.2022.BuildTools
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
  }
  else {
    Write-Warning "MSVC not found: $pathpattern"
    return $null
  }
}

function set_msvc_env_variables {  # Sets the environment variables based on the architecture and MSVC version
  $msvcversion = get_msvc_version
  $filepath = "$global:msvcpath\VC\Tools\MSVC\$msvcversion\bin\Hostx$global:architecture\x$global:architecture"
	[Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$filepath\link.exe;$filepath\cl.exe", "User")
	Write-Host "MSVC added to user PATH environment variable"
}

function install_rustup {
  $result = check_package "Rustup"
  if ($result) {
    winget install Rustlang.Rustup --silent --accept-source-agreements
    Exit
	}
  else {
    winget upgrade --id Rustlang.Rustup --accept-source-agreements
  }
  # Reload the PATH environment variables for this session
  $env:Path = [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "Machine")
  Write-Host "Configuring Rustup..."
  rustup update
  rustup component add rustfmt
  rustup component add clippy
  rustup toolchain install nightly
  rustup component add rustfmt --toolchain nightly
}

function install_protoc {
  if (!(Get-Command protoc -ErrorAction SilentlyContinue)) {

    $protoc_zip = "protoc-$global:protoc_version-win$global:architecture.zip"
    $protoc_folder = "protoc-$global:protoc_version-win$global:architecture"
    $protoc_url = "https://github.com/protocolbuffers/protobuf/releases/download/v$global:protoc_version/$protoc_zip"

    # Download and extract Protoc
    Safe-Download-File $protoc_url -Destination (New-Item -Path "$env:USERPROFILE\Downloads\$protoc_zip" -Force)
    Expand-Archive -Path "$env:USERPROFILE\Downloads\$protoc_zip" -DestinationPath "$env:USERPROFILE\$protoc_folder" -ErrorAction SilentlyContinue
    Remove-Item "$env:USERPROFILE\Downloads\$protoc_zip"

    # Add Protoc installation directory to user PATH environment variable
    [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$env:USERPROFILE\$protoc_folder\bin", "User")
  }
  else {
    Write-Host "Protoc is already installed."
  }
}

function install_cargo_plugins {  # Installs Grcov, protoc components, and cargo components
  cargo install protoc-gen-prost --locked
  cargo install protoc-gen-prost-serde --locked
  cargo install protoc-gen-prost-crate --locked
  cargo install grcov --version $global:grcov_version --locked
  cargo install cargo-sort --locked
  cargo install cargo-nextest --locked
}

function install_llvm {
  $result = check_package "LLVM"
  if ($result) {
    winget install LLVM.LLVM --silent --accept-source-agreements
	}
  else {
    winget upgrade --id LLVM.LLVM --accept-source-agreements
  }
}

function install_openssl {
  $result = check_package "OpenSSL"
  if ($result) {
    winget install ShiningLight.OpenSSL --silent --accept-source-agreements
	}
  else {
    winget upgrade --id ShiningLight.OpenSSL --silent --accept-source-agreements
  }
}

function install_nodejs {
  $result = check_package "Node.js"
  if ($result) {
    winget install OpenJS.NodeJS --silent --accept-source-agreements
	}
  else {
    winget upgrade --id OpenJS.NodeJS --silent --accept-source-agreements
  }
}

function install_python {
  $result = check_package "Python"
  if ($result) {
    winget install Python.Python.3.11 --silent --accept-source-agreements
    # Reload the PATH environment variables for this session
    $env:Path = [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "Machine")
	}
  else {
    winget upgrade --id Python.Python.3.11 --silent --accept-source-agreements
  }
  python -m pip install --upgrade pip
}

function install_pnpm {
  $result = check_package "pnpm"
  if ($result) {
    winget install pnpm.pnpm --silent --accept-source-agreements
	}
  else {
    winget upgrade --id pnpm.pnpm --accept-source-agreements
  }
}

function install_postgresql {
  $result = check_package "PostgreSQL 15"
  $psql_version = winget show -e "PostgreSQL 15" --accept-source-agreements | Select-String Version
  $psql_version = $psql_version.Line.Split(':')[1].Split('.')[0].Trim()
  $psql_path = "$env:PATH;$env:PROGRAMFILES\PostgreSQL\$psql_version\bin"

  if ($result) {
    winget install PostgreSQL.PostgreSQL.15 --silent --accept-source-agreements
    [Environment]::SetEnvironmentVariable("PATH", $psql_path, "User")
  }
  elseif (!(Get-Command psql -ErrorAction SilentlyContinue)) {
    [Environment]::SetEnvironmentVariable("PATH", $psql_path, "User")
  }
  else {
    winget upgrade --id PostgreSQL.PostgreSQL.15 --accept-source-agreements
  }
}

function install_git {
  if (!(Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Git..."
    winget install Git.Git --silent --accept-source-agreements
  }
  else {
    Write-Host "Git is already installed."
  }
}

function install_dotnet {
  if (![System.IO.Path]::IsPathRooted($env:DOTNET_ROOT)) {
    $dotnet_version = $global:dotnet_version.Split('.')[0].Trim()
    Write-Host "Installing Microsoft DotNet..."
    winget install "Microsoft.DotNet.SDK.$dotnet_version" --accept-source-agreements --silent

    $dotnet_version = winget show -e "Microsoft.DotNet.SDK.$dotnet_version" | Select-String -pattern "Version|버전"
    $dotnet_version = $dotnet_version.Line.Split(':')[1].Trim()
    [Environment]::SetEnvironmentVariable("DOTNET_ROOT", "$env:PROGRAMFILES\dotnet", "User")
    [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$env:PROGRAMFILES\dotnet\sdk\$dotnet_version\DotnetTools;$env:USERPROFILE\.dotnet\tools", "User")

    # Reload the PATH environment variables for this session
    $env:Path = [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "Machine")
    Write-Host "User environment variables set for DotNet"
  }
  else {
    Write-Host "Microsoft DotNet is already installed."
  }
}

function install_z3 {
  if (![System.IO.Path]::IsPathRooted($env:Z3_EXE)) {
    Write-Host "Installing Z3..."
    $uri = "z3-$global:z3_version"
    $z3_zip = "z3-$global:z3_version-x$global:architecture-win.zip"
    $z3_filepath = "$env:USERPROFILE\$z3_zip"

    # Download and extract Z3
    Safe-Download-File -Source "https://github.com/Z3Prover/z3/releases/download/$uri/$z3_zip" -Destination (New-Item -Path "$z3_filepath" -Force)
    Expand-Archive $z3_filepath -DestinationPath "$env:USERPROFILE" -ErrorAction SilentlyContinue
    Remove-Item $z3_filepath

    # Create a user environment variable for Z3
    $z3_exe_path = "$env:USERPROFILE\z3-$global:z3_version-x$global:architecture-win\bin\z3.exe"
    [Environment]::SetEnvironmentVariable("Z3_EXE", "$z3_exe_path", "User")
    Write-Host "User environment variable set for Z3"
    }
  else {
    Write-Host "Z3 is already installed."
  }
}

function install_boogie {
  if (![System.IO.Path]::IsPathRooted($env:BOOGIE_EXE)) {
    Write-Host "Installing Boogie..."
    dotnet tool install --global Boogie --version $global:boogie_version
    $boogie_exe_path = "$env:USERPROFILE\.dotnet\tools\boogie.exe"
    [Environment]::SetEnvironmentVariable("BOOGIE_EXE", $boogie_exe_path, "User")
    Write-Host "User environment variables set for Boogie"
  }
  else {
    Write-Host "Boogie is already installed."
  }
}

function install_build_tools {
  Write-Host (build_tools_message)
  install_msvc_build_tools
  install_llvm
  install_openssl
  install_nodejs
  install_pnpm
  install_postgresql
  install_python
  install_protoc
  install_rustup
  install_cargo_plugins
}

function install_move_prover {
  Write-Host (move_prover_message)
  install_dotnet
  install_boogie
  install_z3
  install_git
}

function Safe-Download-File {
    param (
        [Parameter(Mandatory=$true)]
        [string]$Source,

        [Parameter(Mandatory=$true)]
        [string]$Destination
    )

    # Check if the file exists
    if (Test-Path -Path $Destination) {
        Write-Host "File already exists at $Destination. Deleting the existing file..." -ForegroundColor Yellow
        # Remove the existing file
        Remove-Item -Path $Destination -Force
        Write-Host "Existing file deleted." -ForegroundColor Green
    }
    
    # Start the download
    Write-Host "Starting the download from $Source to $Destination..." -ForegroundColor Blue
    Start-BitsTransfer -Source $Source -Destination $Destination
    Write-Host "Download completed successfully." -ForegroundColor Green
}

verify_architecture
check_os
update_versions
check_for_winget

if ($t -or $y) {
    if ($t) {
      $global:user_selection = 't'
      install_build_tools
    }
    if ($y) {
      $global:user_selection = 'y'
      install_move_prover
    }
} else {
  $selection = Read-Host -Prompt (welcome_message)
  $global:user_selection = $selection
  switch ($selection) {
    't' { install_build_tools }
    'y' { install_move_prover }
    default { Write-Host "Invalid option selected. Please enter 't' or 'y'." }
  }
}

Write-Host "Installation complete. Open a new PowerShell session to update the environment variables."
