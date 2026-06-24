[Setup]
AppName=Tadpole
AppVersion=0.1.0
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


[Files]
Source: "dist\tadpole-windows\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Tadpole"; Filename: "{app}\tadpole.exe"
Name: "{commondesktop}\Tadpole"; Filename: "{app}\tadpole.exe"

[Run]
Filename: "{app}\tadpole.exe"; Description: "Launch Tadpole"; Flags: nowait postinstall skipifsilent
