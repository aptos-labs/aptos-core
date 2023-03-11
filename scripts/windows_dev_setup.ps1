# This script installs the necessary dependencies to build in Aptos.

$ErrorActionPreference = 'Stop'

Set-Location (Split-Path -Parent $MyInvocation.MyCommand.Path) | Out-Null; Set-Location '..' -ErrorAction Stop

$global:os = $null
$global:architecture = $null
$global:msvcpath = $null

function welcome_message {
    $welcome_message = "`nWelcome to Aptos!

    This script will download and install the necessary dependencies needed to build Aptos Core.

    These tools will be installed if not found on your system:
        
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
        * LLVM
        * CMake
        * OpenSSL
        * NodeJS
        * NPM
        * PostgreSQL
        * Grcov"


    Write-Host $welcome_message
    Write-Host "`nPress any key to begin installation..."
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
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
     Invoke-WebRequest -Uri "https://www.nuget.org/api/v2/package/Microsoft.UI.Xaml/2.8.2" -OutFile "Microsoft.UI.Xaml.2.8.2.nupkg.zip" -ErrorAction SilentlyContinue
     Expand-Archive "Microsoft.UI.Xaml.2.8.2.nupkg.zip" -ErrorAction SilentlyContinue
     while ((Get-Item "Microsoft.UI.Xaml.2.8.2.nupkg.zip").Length -lt 19MB) {
         Start-Sleep -Seconds 1
     }

    if ($global:architecture -eq "64") {
        # Install x64 dependencies (VCLibs and XAML)
        Invoke-WebRequest -Uri "https://aka.ms/Microsoft.VCLibs.x64.14.00.Desktop.appx" -OutFile "Microsoft.VCLibs.x64.14.00.Desktop.appx" -ErrorAction SilentlyContinue
        while ((Get-Item "Microsoft.VCLibs.x64.14.00.Desktop.appx").Length -lt 6MB) {
            Start-Sleep -Seconds 1
     }
     Add-AppxPackage "Microsoft.VCLibs.x64.14.00.Desktop.appx" -ErrorAction SilentlyContinue
     Add-AppxPackage "Microsoft.UI.Xaml.2.8.2.nupkg\tools\AppX\x64\Release\Microsoft.UI.Xaml.2.8.appx" -ErrorAction SilentlyContinue
    }
    elseif ($global:architecture -eq "86") {
        # Install x86 dependencies (VCLibs and XAML)
        Invoke-WebRequest -Uri "https://aka.ms/Microsoft.VCLibs.x86.14.00.Desktop.appx" -OutFile "Microsoft.VCLibs.x86.14.00.Desktop.appx" -ErrorAction SilentlyContinue
        while ((Get-Item "Microsoft.VCLibs.x86.14.00.Desktop.appx").Length -lt 5.5MB) {
            Start-Sleep -Seconds 1
        }
     Add-AppxPackage "Microsoft.VCLibs.x86.14.00.Desktop.appx" -ErrorAction SilentlyContinue
     Add-AppxPackage "Microsoft.UI.Xaml.2.8.2.nupkg\tools\AppX\x86\Release\Microsoft.UI.Xaml.2.8.appx" -ErrorAction SilentlyContinue
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
    [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;%LOCALAPPDATA%\Microsoft\WindowsApps", "User")
    
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

function check_package {
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
	winget upgrade --id OpenJS.NodeJS -silent
  }
}

function install_python {
  $result = check_package "Python"
  if ($result) {
	winget install Python.Python.3.11 --silent
	}
  else {
	winget upgrade --id Python.Python.3.11 -silent
  }
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

welcome_message
verify_architecture
check_os
install_msvc_build_tools
install_llvm
install_openssl
install_nodejs
install_pnpm
install_postgresql
install_rustup
Write-Host "Finished..."
