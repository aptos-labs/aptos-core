# This script installs the necessary dependencies to build in Aptos.

param (
    [switch]$t,
    [switch]$y
)

$ErrorActionPreference = 'Stop'

Set-Location (Split-Path -Parent $MyInvocation.MyCommand.Path) | Out-Null; Set-Location '..' -ErrorAction Stop

$global:user_selection = $null
$global:os = $null
$global:architecture = $null
$global:msvcpath = $null
$global:grcov_version = "0.8.2"
$global:protoc_version = "21.4"
$global:cvc5_version = "0.0.8"
$global:dotnet_version = "6.0.407"
$global:z3_version = "4.11.2"
$global:boogie_version = "2.15.8"


function welcome_message {
    $message = "`nWelcome to Aptos!
    `nThis script will download and install the necessary dependencies for Aptos Core based on your selection:
      * Install Aptos build tools: t
      * Install Move Prover tools: y`n
      Selection"
    
    return $message
}

function build_tools_message {
    $message = "`nYou selected option 't'.
    `nThe following dependencies needed to build Aptos Core will be downloaded and installed if not found on your system:
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
      * schemathesis
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
    `nThe following dependencies needed to use the Move Prover will be downloaded and installed if not found on your system:
    * Dotnet
    * Z3
    * Boogie
    * CVC5"
    return $message
}

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
		Write-Host "Supported Windows OS detected"
		$global:os = "Windows Server 2022"
		check_for_winget
	}
	else {
		Write-Host "Unsupported Windows OS detected. Stopping script..."
		Exit
	}
}

function install_winget {
    # Download and extract XAML dependency
    Invoke-WebRequest -Uri "https://www.nuget.org/api/v2/package/Microsoft.UI.Xaml/2.7.1" -OutFile "Microsoft.UI.Xaml.2.7.1.nupkg.zip" -ErrorAction SilentlyContinue
    while ((Get-Item "Microsoft.UI.Xaml.2.7.1.nupkg.zip").Length -lt 18MB) {
        Start-Sleep -Seconds 1
    }
    Expand-Archive "Microsoft.UI.Xaml.2.7.1.nupkg.zip" -ErrorAction SilentlyContinue

    if ($global:architecture -eq "64") {
        # Install x64 dependencies (VCLibs and XAML)
        Invoke-WebRequest -Uri "https://aka.ms/Microsoft.VCLibs.x64.14.00.Desktop.appx" -OutFile "Microsoft.VCLibs.x64.14.00.Desktop.appx" -ErrorAction SilentlyContinue
        while ((Get-Item "Microsoft.VCLibs.x64.14.00.Desktop.appx").Length -lt 6MB) {
            Start-Sleep -Seconds 1
        }
        Add-AppxPackage "Microsoft.VCLibs.x64.14.00.Desktop.appx" -ErrorAction SilentlyContinue
        Add-AppxPackage "Microsoft.UI.Xaml.2.7.1.nupkg\tools\AppX\x64\Release\Microsoft.UI.Xaml.2.7.appx" -ErrorAction SilentlyContinue
    }
    elseif ($global:architecture -eq "86") {
        # Install x86 dependencies (VCLibs and XAML)
        Invoke-WebRequest -Uri "https://aka.ms/Microsoft.VCLibs.x86.14.00.Desktop.appx" -OutFile "Microsoft.VCLibs.x86.14.00.Desktop.appx" -ErrorAction SilentlyContinue
        while ((Get-Item "Microsoft.VCLibs.x86.14.00.Desktop.appx").Length -lt 5.5MB) {
            Start-Sleep -Seconds 1
        }
        Add-AppxPackage "Microsoft.VCLibs.x86.14.00.Desktop.appx" -ErrorAction SilentlyContinue
        Add-AppxPackage "Microsoft.UI.Xaml.2.7.1.nupkg\tools\AppX\x86\Release\Microsoft.UI.Xaml.2.7.appx" -ErrorAction SilentlyContinue
    }

    # Install WinGet
    Invoke-WebRequest -Uri "https://github.com/microsoft/winget-cli/releases/download/v1.4.10173/Microsoft.DesktopAppInstaller_8wekyb3d8bbwe.msixbundle" -OutFile "msftwinget.msixbundle" -ErrorAction SilentlyContinue
    while ((Get-Item "msftwinget.msixbundle").Length -lt 13.5MB) {
        Start-Sleep -Seconds 1
    }
    Invoke-WebRequest -Uri "https://github.com/microsoft/winget-cli/releases/download/v1.4.10173/3463fe9ad25e44f28630526aa9ad5648_License1.xml" -OutFile "license.xml" -ErrorAction SilentlyContinue
    while ((Get-Item "license.xml").Length -lt 1KB) {
        Start-Sleep -Seconds 1
    }
    Add-AppxProvisionedPackage -Online -PackagePath "msftwinget.msixbundle" -LicensePath "license.xml" -ErrorAction SilentlyContinue

    # Add WinGet directory to the user's PATH environment variable
    [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$env:%LOCALAPPDATA%\Microsoft\WindowsApps", "User")

    Write-Host "Please restart your system to ensure WinGet is setup correctly. Afterward, re-run the script."
}

function check_for_winget {
    if (Get-Command "winget" -ErrorAction SilentlyContinue) {
    Write-Host "WinGet is already installed."
    } 
    else {
    Write-Host "Installing WinGet..."
    install_winget
    }
}

function check_package { # Checks for packages installed with winget or typical installers
  param(
    [string]$package
  )
  if ((winget list --name $package --accept-source-agreements) -match 'No installed package found matching input criteria.') {
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

function check_non_winget_or_installer_package {  # Checks for packages that were manually installed via an archive/zip file
    param(
      [string]$package
    )

    $env_var = [Environment]::GetEnvironmentVariable("PATH", "User").Split(";")

    foreach ($dir in $env_var) {
        if ($dir -like "*$package*") {
            Write-Host "$package is already installed"
            return $true
        }
        else {
            Write-Host "Installing $package..."
            return $false
        }
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
    } else {
        Write-Warning "MSVC not found: $pathpattern"
        return $null
    }
}

function set_msvc_env_variables {  # Sets the environment variables based on the architecture and MSVC version
  $msvcversion = get_msvc_version
  $filepath = "$global:msvcpath\VC\Tools\MSVC\$msvcversion\bin\Hostx$global:architecture\x$global:architecture"
	[Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$filepath\link.exe;$filepath\cl.exe", "User")
	Write-Host "Environment variables set"
}

function install_rustup {
  $result = check_package "Rustup"
  if ($result) {
	winget install Rustlang.Rustup --silent
	Exit
	}
  else {
	winget upgrade --id Rustlang.Rustup
  }
  Write-Host "Configuring Rustup..."
  rustup update
  rustup component add rustfmt
  rustup component add clippy
  rustup toolchain install nightly
  rustup component add rustfmt --toolchain nightly
}

function install_protoc {
    $exists = check_non_winget_or_installer_package "Protoc"
    if (!$exists) {
        if ($global:os -eq "64") {
            # Download and extract the 64-bit version of Protoc
            Invoke-WebRequest -Uri "https://github.com/protocolbuffers/protobuf/releases/download/v$global:protoc_version/protoc-$global:protoc_version-win64.zip" -OutFile "protoc-$global:protoc_version-win64.zip" -ErrorAction SilentlyContinue
            while ((Get-Item "protoc-$global:protoc_version-win64.zip").Length -lt 2MB) {
                Start-Sleep -Seconds 1
            }
            Expand-Archive -Path "protoc-$global:protoc_version-win64.zip" -DestinationPath "$env:USERPROFILE\protoc-$global:protoc_version-win64" -ErrorAction SilentlyContinue
        }
        else {
            # Download and extract the 32-bit version of Protoc
            Invoke-WebRequest -Uri "https://github.com/protocolbuffers/protobuf/releases/download/v$global:protoc_version/protoc-$global:protoc_version-win32.zip" -OutFile "protoc-$global:protoc_version-win32.zip" -ErrorAction SilentlyContinue
            while ((Get-Item "protoc-$global:protoc_version-win32.zip").Length -lt 2MB) {
                Start-Sleep -Seconds 1
            }
            Expand-Archive -Path "protoc-$global:protoc_version-win32.zip" -DestinationPath "$env:USERPROFILE\protoc-$global:protoc_version-win32" -ErrorAction SilentlyContinue
        }
        
        # Add the Protoc installation directory to the user's PATH environment variable
        [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$env:USERPROFILE\protoc-$global:protoc_version-win64\bin", "User")
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
	winget install LLVM.LLVM --silent
	}
  else {
	winget upgrade --id LLVM.LLVM
  }
}

function install_openssl {
  $result = check_package "OpenSSL"
  if ($result) {
	winget install ShiningLight.OpenSSL --silent
	}
  else {
	winget upgrade --id ShiningLight.OpenSSL --silent
  }
}

function install_nodejs {
  $result = check_package "Node.js"
  if ($result) {
	winget install OpenJS.NodeJS --silent
	}
  else {
	winget upgrade --id OpenJS.NodeJS --silent
  }
}

function install_python {
  $result = check_package "Python"
  if ($result) {
	winget install Python.Python.3.11 --silent
	}
  else {
	winget upgrade --id Python.Python.3.11 --silent
  }
  python -m pip install --upgrade pip
  python -m pip install schemathesis
}

function install_pnpm {
  $result = check_package "pnpm"
  if ($result) {
	winget install pnpm.pnpm --silent
	}
  else {
    winget upgrade --id LLVM.LLVM
  }
}

function install_postgresql {
  $result = check_package "PostgreSQL"
  if ($result) {
	winget install PostgreSQL.PostgreSQL --silent
	}
  else {
	winget upgrade --id LLVM.LLVM
   }
}

function existing_package {
	Write-Host "This package is already installed."
}

function install_git {
  if (Get-Command git -ErrorAction SilentlyContinue) {
    Write-Host "Installing Git..."
    winget install Git.Git --silent
  } 
  else {
    winget upgrade --id Git.Git
  }
}

function install_cvc5 {
  if (![System.IO.Path]::IsPathRooted($env:CVC5_EXE)) {
    Write-Host "Installing CVC5..."
    # Download and extract the 64-bit version of Protoc
    $cvc5_exe = "$env:USERPROFILE\cvc5-$global:cvc5_version\cvc5-Win64.exe"
    Write-Host $path
    Invoke-WebRequest -Uri "https://github.com/cvc5/cvc5/releases/download/cvc5-$global:cvc5_version/cvc5-Win64.exe" -OutFile (New-Item -Path "$cvc5_exe" -Force) -ErrorAction SilentlyContinue
    [Environment]::SetEnvironmentVariable("CVC5_EXE", "$cvc5_exe", "User") 
    Write-Host "User environment variables set for CVC5"
  }
  elseif ($global:architecture -eq "86") {
    Write-Host "Only 64-bit systems can install CVC5"
  }
  else {
    Write-Host "CVC5 is already installed."
  }
}

function install_dotnet {
  if (![System.IO.Path]::IsPathRooted($env:DOTNET_ROOT)) {
    Write-Host "Installing Microsoft DotNet..."
    winget install "Microsoft.DotNet.SDK.6" --accept-source-agreements --silent
    [Environment]::SetEnvironmentVariable("DOTNET_ROOT", "$env:PROGRAMFILES\dotnet", "User")
    [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$env:PROGRAMFILES\dotnet\sdk\$global:dotnet_version\DotnetTools;;$env:PATH;$env:USERPROFILE\.dotnet\tools", "User")
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
    # Download and extract the 64-bit version of Protoc
    Invoke-WebRequest -Uri "https://github.com/Z3Prover/z3/releases/download/$uri/$z3_zip" -OutFile (New-Item -Path "$z3_filepath" -Force) -ErrorAction SilentlyContinue
    if ($global:architecture -eq "64") {
      while ((Get-Item "$z3_filepath").Length -lt 52MB) {
        Start-Sleep -Seconds 1
      }
    }
    else {
      while ((Get-Item "$z3_filepath").Length -lt 40MB) {
        Start-Sleep -Seconds 1
      }
    }
    Expand-Archive "$z3_filepath" -ErrorAction SilentlyContinue
    $z3_exe = "$env:USERPROFILE\z3-$global:z3_version-x$global:architecture-win\z3-$global:z3_version-x$global:architecture-win\bin\z3.exe"
    [Environment]::SetEnvironmentVariable("Z3_EXE", "$z3_exe", "User")    
    Write-Host "User environment variable set for Z3"
    }
  else {
    Write-Host "Z3 is already installed."
  }
}

function install_boogie {
    if (![System.IO.Path]::IsPathRooted($env:BOOGIE_EXE)) {
      Write-Host "Installing boogie..."
      dotnet tool install --global Boogie --version $global:boogie_version
      $boogie_exe = "$env:USERPROFILE\.dotnet\tools\boogie.exe"
      [Environment]::SetEnvironmentVariable("BOOGIE_EXE", $boogie_exe, "User")
      Write-Host "User environment variables set for Boogie"
    } 
    else {
      Write-Host "Boogie is already installed."
    }
}

function install_build_tools {
  Write-Host (build_tools_message)
  verify_architecture
  check_os
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
  Write-Host "Installation complete. Open a new PowerShell session to update the environment variables."
}

function install_move_prover {
  Write-Host (move_prover_message)
  verify_architecture
  check_os
  install_cvc5
  install_dotnet
  install_z3
  install_boogie
  install_git
  Write-Host "Installation complete. Open a new PowerShell session to update the environment variables."
}

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
Write-Host "Finished..."
