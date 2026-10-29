; Build with scripts/installer.ps1 after scripts/package.ps1.
#ifndef PopyVersion
  #error PopyVersion is required
#endif
#ifndef PackageRoot
  #error PackageRoot is required
#endif
#ifndef OutputRoot
  #error OutputRoot is required
#endif

[Setup]
AppId={{778ECA55-84AB-4DD9-96FA-420EB336F851}
AppName=Popy
AppVersion={#PopyVersion}
AppPublisher=Popy contributors
AppPublisherURL=https://github.com/boranseasonnew/Popy
AppSupportURL=https://github.com/boranseasonnew/Popy/issues
DefaultDirName={localappdata}\Programs\Popy
DefaultGroupName=Popy
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#OutputRoot}
OutputBaseFilename=Popy-{#PopyVersion}-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
WizardBackColor=$F4F8FF
LicenseFile={#PackageRoot}\LICENSE
UninstallDisplayIcon={app}\popy-desktop.exe
CloseApplications=yes
RestartApplications=no
SetupLogging=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#PackageRoot}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Popy"; Filename: "{app}\popy-desktop.exe"; WorkingDir: "{app}"
Name: "{userdesktop}\Popy"; Filename: "{app}\popy-desktop.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\popy-desktop.exe"; Description: "{cm:LaunchProgram,Popy}"; Flags: nowait postinstall skipifsilent

; No recursive uninstall deletion: runtime, preferences and exports created by
; the user remain in the application folder after uninstall.
