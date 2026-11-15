#define AppName "Wive"
#define AppId "{{CCBD571D-8A89-4A77-9C69-E1971EEB7096}}"
#define AppVersion GetEnv("WIVE_VERSION")

[Setup]
AppId={#AppId}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Aleverio / Myth Devi Studio
DefaultDirName={localappdata}\Programs\Wive
DefaultGroupName=Wive
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64
OutputDir=..\..\dist
OutputBaseFilename=Wive-Setup-x64
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\wive.exe

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Files]
Source: "..\..\target\release\wive.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\icons\LICENSE"; DestDir: "{app}\licenses"; DestName: "lucide-icons-LICENSE"

[Icons]
Name: "{autoprograms}\Wive"; Filename: "{app}\wive.exe"
Name: "{autodesktop}\Wive"; Filename: "{app}\wive.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\wive.exe"; Description: "Launch Wive"; Flags: nowait postinstall skipifsilent
