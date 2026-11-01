# RetroRadar

Consola espacial retro para **Windows nativo y Rust**.
Repositorio de TheWinterLynx, distribuido bajo GPL-3.0-or-later.

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
- Globo HD: día 16K y noche 8K reales, niveles de calidad y reducción según GPU.

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

## Calidad del globo

Para máxima calidad, después del gate:

```powershell
.\target\release\retroradar.exe --earth-quality high --no-record
```

| Opción `--earth-quality` | Día | Noche |
|---|---|---|
| `low` | 4096 × 2048 | 2048 × 1024 |
| `medium` | 8192 × 4096 | 4096 × 2048 |
| `high` | 16384 × 8192 | 8192 × 4096 |
| `auto` (por defecto) | Hasta 8192 × 4096 | Hasta 4096 × 2048 |

Todos los niveles respetan el límite del dispositivo; el renderizado por software
se limita a 2048 × 1024 de día y 1024 × 512 de noche. La terminal y SYS muestran
la resolución efectiva. Cambiar de nivel requiere reiniciar. `high` necesita unos
896 MiB de texturas con mipmaps, aparte de los demás recursos gráficos; `auto`
es conservador y no mide la VRAM libre. El detalle resulta más visible al acercar
la cámara y con el tema Modern, que muestra el mapa en color. El arrastre y las
flechas reducen su sensibilidad al acercar el globo. VIEW/NADIR comparte las mismas
texturas; las costas superpuestas se atenúan al ampliarlas para evitar escalones.

Las imágenes NASA se han reducido desde originales mayores, sin ampliar píxeles.
Son mapas estáticos de 2004 (día) y 2012 (noche). Véase
[Globo HD](docs/R01-HD-EARTH-v1.0.md) para fuentes, memoria y aceptación.

## Zoom con detalle regional

El globo permite acercarse hasta **10 km sobre la superficie**. En VIEW/NADIR,
la rueda o `+`/`-` cambian el zoom óptico entre **1× y 100×**, manteniendo la altitud
real del vehículo. El control se hace más lento según te acercas.

Las zonas cercanas cargan teselas **Sentinel-2 de EOX, edición 2016**, hasta unos
**10 m por píxel** según encuadre y cobertura. Se muestran progresivamente; el mapa
global cubre las teselas que todavía no han llegado. La primera visita necesita
conexión y puede tardar varios segundos. Las zonas descargadas quedan en caché.
Son fotografías históricas diurnas, con sombreado según el Sol; las luces nocturnas
siguen usando el mapa global. No hay relieve 3D ni un nivel de detalle ilimitado.

En VIEW puedes pausar con **Espacio** para inspeccionar una zona sin que el vehículo
siga avanzando. El contador `DETAIL` muestra nivel, resolución de la cuadrícula y
teselas cargadas. `--offline` usa solo la caché; `--no-earth-detail` conserva solo
las texturas globales. El caché de detalle se limita aproximadamente a 256 MiB y
la textura regional ocupa 16 MiB adicionales de GPU. Créditos y licencia CC BY 4.0
se muestran sobre la vista. [Detalle regional](docs/R01-REGIONAL-EARTH-v1.0.md).

## Datos y configuración

```powershell
.\target\release\retroradar.exe --data-dir .\portable-data doctor
```

La ruta anterior contiene `config\config.toml`, `cache` y `data\retroradar.db`.
La aplicación crea una plantilla comentada al arrancar normalmente. `doctor` solo
lee. Sin `--data-dir`, se usan los directorios de aplicación Windows del usuario,
que muestra `doctor`. Los datos se guardan exclusivamente en los directorios de RetroRadar.
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
[manual de uso](docs/USAGE-v1.0.md).

[Roadmap](docs/ROADMAP.md) · [Avisos legales](NOTICE.md) ·
[Arquitectura](docs/architecture.md) · [Contrato de trabajo](AGENTS.md)
