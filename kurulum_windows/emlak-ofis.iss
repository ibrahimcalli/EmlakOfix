; Emlak Ofis — Windows kurulum sihirbazı (Inno Setup 6)
; Derleme: iscc /DSurum=1.4.0 emlak-ofis.iss   (emlak-ofis.exe bu klasörün bir üstünde target\release içinde olmalı)
#ifndef Surum
  #define Surum "1.4.0"
#endif
[Setup]
AppId={{6F1B7C52-3A8E-4D0B-9C47-5E1A0F2B7D01}
AppName=Emlak Ofis
AppVersion={#Surum}
AppPublisher=Emlak Ofis
AppPublisherURL=https://emlakfethiye.com.tr
DefaultDirName={autopf}\Emlak Ofis
DefaultGroupName=Emlak Ofis
DisableProgramGroupPage=yes
OutputBaseFilename=EmlakOfis-Kurulum-{#Surum}
OutputDir=..\cikti
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\emlak-ofis.exe
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ShowLanguageDialog=no

[Languages]
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"

[Tasks]
Name: "desktopicon"; Description: "Masaüstüne kısayol oluştur"; Flags: unchecked
Name: "firewall"; Description: "Bu bilgisayar ANA BİLGİSAYAR olacaksa güvenlik duvarında Emlak Ofis'e izin ver (yönetici izni ister)"; Flags: unchecked

[Files]
Source: "..\target\release\emlak-ofis.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Emlak Ofis"; Filename: "{app}\emlak-ofis.exe"
Name: "{group}\Emlak Ofis'i kaldır"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Emlak Ofis"; Filename: "{app}\emlak-ofis.exe"; Tasks: desktopicon

[Run]
Filename: "netsh"; Parameters: "advfirewall firewall add rule name=""Emlak Ofis"" dir=in action=allow program=""{app}\emlak-ofis.exe"" enable=yes profile=private"; Flags: runhidden; Tasks: firewall
Filename: "{app}\emlak-ofis.exe"; Description: "Emlak Ofis'i başlat"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "netsh"; Parameters: "advfirewall firewall delete rule name=""Emlak Ofis"""; Flags: runhidden; RunOnceId: "fwdel"

; Veriler (ayarlar, veritabanı, yedekler) kullanıcı klasöründe tutulur; kaldırma işlemi bunları SİLMEZ.
