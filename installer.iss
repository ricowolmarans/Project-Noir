[Setup]
AppName=Noir
AppVersion=0.1.0
AppPublisher=root
DefaultDirName={autopf}\Noir
DefaultGroupName=Noir
OutputBaseFilename=Noir-Setup
OutputDir=installer-out
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequiredOverridesAllowed=dialog
UninstallDisplayName=Noir

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "target\release\noir.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Noir"; Filename: "{app}\noir.exe"
Name: "{autodesktop}\Noir"; Filename: "{app}\noir.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\noir.exe"; Description: "Launch Noir"; Flags: nowait postinstall skipifsilent