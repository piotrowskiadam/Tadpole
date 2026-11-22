[Setup]
AppName=Tadpole
AppVersion=0.5.0
DefaultDirName={autopf}\Tadpole
DefaultGroupName=Tadpole
UninstallDisplayIcon={app}\tadpole.exe
SetupIconFile=tadpole.ico
WizardImageFile=installer_wizard.png
WizardSmallImageFile=installer_small.png
Compression=lzma2
SolidCompression=yes
OutputDir=dist
OutputBaseFilename=TadpoleSetup
DisableWelcomePage=no
DisableDirPage=no
ChangesAssociations=yes


[Files]
Source: "dist\tadpole-windows\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs
Source: "tadpole.ico"; DestDir: "{app}"; Flags: ignoreversion

[Registry]
Root: HKA; Subkey: "Software\Classes\.tadpole"; ValueType: string; ValueName: ""; ValueData: "Tadpole.Project"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.tadpole"; ValueType: string; ValueName: "Content Type"; ValueData: "application/x-tadpole-project"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\Tadpole.Project"; ValueType: string; ValueName: ""; ValueData: "Tadpole Project"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Tadpole.Project\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\tadpole.ico,0"
Root: HKA; Subkey: "Software\Classes\Tadpole.Project\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\tadpole.exe"" ""%1"""

[Icons]
Name: "{group}\Tadpole"; Filename: "{app}\tadpole.exe"; IconFilename: "{app}\tadpole.ico"
Name: "{commondesktop}\Tadpole"; Filename: "{app}\tadpole.exe"; IconFilename: "{app}\tadpole.ico"

[Run]
Filename: "{app}\tadpole.exe"; Description: "Launch Tadpole"; Flags: nowait postinstall skipifsilent
