#define MyAppName "GoodbyeDPI Manager"
#define MyAppVersion "1.04"
#define MyAppExeName "goodbyedpi-manager.exe"

[Setup]
AppId={{A3A26543-92F4-4C89-A221-BE7717746FE4}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher=H Son
DefaultDirName={autopf}\GoodbyeDPI Manager
DisableProgramGroupPage=yes
PrivilegesRequired=admin
OutputDir=..\..\..\..\outputs
OutputBaseFilename=GoodbyeDPI-Manager-All-in-One-1.04
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayName={#MyAppName}
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\engine\goodbyedpi.exe"; DestDir: "{app}\engine"; Flags: ignoreversion
Source: "..\engine\WinDivert.dll"; DestDir: "{app}\engine"; Flags: ignoreversion
Source: "..\engine\WinDivert64.sys"; DestDir: "{app}\engine"; Flags: ignoreversion
Source: "..\engine\licenses\*"; DestDir: "{app}\engine\licenses"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Tạo biểu tượng ngoài Desktop"; GroupDescription: "Biểu tượng bổ sung:"; Flags: checkedonce

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Mở {#MyAppName}"; Verb: "runas"; Flags: nowait postinstall skipifsilent shellexec

[Code]
const
  ServiceKey = 'Software\GoodbyeDPI Manager';

function ServiceExists: Boolean;
var
  ResultCode: Integer;
begin
  Result := Exec(ExpandConstant('{sys}\sc.exe'), 'query GoodbyeDPI', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) and (ResultCode = 0);
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  OwnedPath: String;
  ResultCode: Integer;
begin
  if CurStep = ssPostInstall then begin
    if not ServiceExists then begin
      if not Exec(ExpandConstant('{app}\{#MyAppExeName}'), '--install-service', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) or (ResultCode <> 0) then
        RaiseException('Could not create GoodbyeDPI service. Error: ' + IntToStr(ResultCode));
      RegWriteStringValue(HKLM, ServiceKey, 'OwnedServicePath', ExpandConstant('{app}\engine\goodbyedpi.exe'));
      Exec(ExpandConstant('{sys}\sc.exe'), 'start GoodbyeDPI', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    end else if RegQueryStringValue(HKLM, ServiceKey, 'OwnedServicePath', OwnedPath) and
      (CompareText(OwnedPath, ExpandConstant('{app}\engine\goodbyedpi.exe')) = 0) then begin
      if not Exec(ExpandConstant('{app}\{#MyAppExeName}'), '--migrate-service', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) or (ResultCode <> 0) then
        MsgBox('Could not update the GoodbyeDPI service. Stop it and apply a preset in Settings.', mbError, MB_OK);
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  OwnedPath: String;
  ResultCode: Integer;
begin
  if (CurUninstallStep = usUninstall) and
     RegQueryStringValue(HKLM, ServiceKey, 'OwnedServicePath', OwnedPath) and
     (CompareText(OwnedPath, ExpandConstant('{app}\engine\goodbyedpi.exe')) = 0) then begin
    Exec(ExpandConstant('{sys}\sc.exe'), 'stop GoodbyeDPI', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Exec(ExpandConstant('{sys}\sc.exe'), 'delete GoodbyeDPI', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    RegDeleteKeyIncludingSubkeys(HKLM, ServiceKey);
  end;
end;

