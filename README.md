# RetroRadar

Consola espacial retro para **Windows nativo y Rust**, basada en
[mctl de Liam Droog](https://github.com/LiamDroog/mctl). Repositorio independiente,
sin fork. Conserva GPL-3.0-or-later, créditos y licencias de recursos originales.

## R01: base Windows y primeras ampliaciones

Conserva las once páginas: MASTER, TRAJ, GNC, EPS, TCS/ECLSS, COMM, PASS, PROX,
VIEW, SYS y OPS; globo orbital 3D, trazas, predicción de pases, ISS/Dragon/Hubble/
JWST/Roman, telemetría ISS, maniobras, grabación SQLite, replay y exportación CSV.
También conserva los cuatro temas y efectos CRT originales.

El código de la aplicación es Rust. **SQLite es la excepción nativa autorizada**.
La criptografía HTTPS/WSS usa RustCrypto en lugar de ring. El proveedor publicado
`rustls-rustcrypto 0.0.2-alpha` es experimental y su autor desaconseja producción;
esta limitación sigue abierta aunque las pruebas TLS pasen. No se desactivan las
validaciones de certificados ni nombres de servidor. Véase la
[política de dependencias](docs/RUST-DEPENDENCIES-v1.0.md).

Añadido en R01:

- Ejecutable `retroradar.exe`, ventana RetroRadar y directorios propios.
- Aislamiento de X11 y GeoClue/D-Bus a Linux. Windows usa la ventana nativa de winit
  y los adaptadores gráficos de wgpu. La localización se configura mediante sitios;
  no se simula un servicio GeoClue en Windows.
- `--data-dir <carpeta>`: configuración, caché y grabaciones portátiles.
- `doctor`: valida configuración y teclas, muestra plataforma y rutas sin red,
  GPU ni escrituras. No verifica conectividad ni controladores.
- `--kiosk`: alternancia de escenas a resolución de escritorio, sin activar `--pi`.
- Diagnósticos en terminal, sin ficheros de log.

Estado: implementación R01 en rama; aceptación Windows y revisión visual del
propietario pendientes. Las pantallas y datos externos heredados no se consideran
revalidados por el mero hecho de importarlos.

## Preparación Windows

Instalar **Git for Windows**, **Rust mediante rustup** y **Visual Studio 2022 Build
Tools**, con la carga **Desktop development with C++**, herramientas MSVC y Windows
SDK. Se necesitan para enlazar el ejecutable Rust y compilar SQLite; no hay
código de aplicación C/C++. Reiniciar la terminal después de instalar. Se fija Rust 1.95.0 en
`rust-toolchain.toml`; rustup lo descarga al ejecutar Cargo. Usar toolchain MSVC,
no WSL. La primera compilación requiere Internet para descargar dependencias.

Desde la carpeta donde quieras guardar el proyecto, en PowerShell:

```powershell
git clone --branch agent/r01-windows-foundation https://github.com/TheWinterLynx/retroradar.git; if ($LASTEXITCODE -ne 0) { throw 'CLONE FAILED' }; Set-Location retroradar; powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\gate-r01-v1.0.ps1; if ($LASTEXITCODE -ne 0) { throw 'R01 GATE FAILED' }
```

El script muestra el commit probado y para al primer fallo: limpieza del checkout,
prerrequisitos, formato, política de dependencias Windows, tests del workspace con `-Dwarnings`, compilación release,
doctor aislado y ventana offline durante 120 frames. Sin datos en caché esa ventana
puede indicar NO DATA: es correcto. `-SkipGpu` solo permite validar código y no
constituye aceptación gráfica. No guarda informes ni logs.

Después del gate automatizado, abrir la aplicación conectada y revisar F1–F11,
Tab, los cuatro temas con T, selección con V, redimensionado, globo, datos y quiosco:

```powershell
.\target\release\retroradar.exe --data-dir .\portable-data --no-record
```

```powershell
.\target\release\retroradar.exe --data-dir .\portable-data --kiosk --theme retrowave --no-record
```

Cerrar con Q. Tras la revisión satisfactoria, comunicar **R01 FULL GATE GREEN**
junto con el commit mostrado. Ese resultado autoriza integrar esa versión en main
y continuar el siguiente slice. No se integra antes de tu gate; no se abren PRs.
Actions se limita a `cargo fmt --all -- --check`.

## Datos y configuración

```powershell
.\target\release\retroradar.exe --data-dir .\portable-data doctor
```

La ruta anterior contiene `config\config.toml`, `cache` y `data\retroradar.db`.
La aplicación crea una plantilla comentada al arrancar normalmente. `doctor` solo
lee. Sin `--data-dir`, se usan los directorios de aplicación Windows del usuario,
que muestra `doctor`. No se migran ni modifican datos existentes de mctl.
Una ruta explícita `[store].path` sigue teniendo prioridad sobre el destino portátil.
`portable-data` está excluido de Git. Otras carpetas personales deben quedar fuera
del checkout para mantener el gate limpio.

Para localizar el observador, editar la configuración creada por la aplicación;
se pueden configurar sitios `[[site]]` y el sitio activo según la plantilla.
La consulta IP permanece desactivada por defecto. Los datos no disponibles muestran
NO DATA; se mantienen los indicadores de estimación, TLE, planes y antigüedad.

`check` consulta servicios externos y puede fallar por disponibilidad de terceros;
no forma parte del gate offline. `--offline`, `--replay`, `export`, `shot`, `bench`
y `isslive` siguen disponibles; consultar `--help` y el
[manual original](docs/UPSTREAM-README.md) para la funcionalidad heredada.

[Roadmap](docs/ROADMAP.md) · [Procedencia](docs/UPSTREAM.md) ·
[Arquitectura heredada](docs/architecture.md) · [Contrato de trabajo](AGENTS.md)
