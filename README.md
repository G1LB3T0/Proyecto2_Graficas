# Proyecto 2 - Ray Tracing en CPU

## Que es esto

Este proyecto es un raytracer hecho en CPU usando Rust y raylib. Esta bastante bueno porque renderiza un mundo de Minecraft con iluminación realista, lámparas que de verdad alumbran, agua con reflejos y un cielo con estrellas.

## Que necesitas para que jale

- Rust (la versión más nueva que tengas)
- Las librerías que están en el Cargo.toml (se instalan solitas)

## Como hacer que funcione

1. Clona este repo o descárgalo 
2. Abre una terminal en la carpeta del proyecto
3. Ejecuta estos comandos:

```bash
cargo build --release
cargo run --release
```

Si quieres que vaya más rápido, usa `--release`, sino va a estar lento.

## Como se usa

### Controles de cámara
- **Mouse izquierdo + arrastrar**: Rotar la cámara alrededor de la isla
- **Rueda del mouse**: Acercar o alejar la cámara
- **R**: Resetear la cámara a la posición original

### Controles de luz
- **J/L**: Mover la luz para la izquierda/derecha (yaw)
- **I/K**: Mover la luz para arriba/abajo (pitch)
- **U/O**: Cambiar el radio de la luz
- **P**: Hacer que la luz gire sola
- **T**: Resetear la luz

### Otros controles
- **F5**: Cambiar entre día y noche (aquí se ve lo bueno de las lámparas)
- **F6**: Prender/apagar los reflejos del agua
- **Z/X**: Subir/bajar toda la isla
- **C**: Resetear la altura de la isla
- **H**: Mostrar/ocultar el HUD
- **F1**: Cambiar resolución del render (baja, media, alta)

## Que tiene de especial

### Ray Tracing en CPU
La cosa está hecha para correr solo en CPU usando múltiples hilos. Nada de GPU ni OpenGL, puro Rust machacando números.

### Mundo de Voxeles
El mundo se carga desde archivos de texto en la carpeta `assets/layers/`. Cada archivo es una capa de 16x16 bloques.

### Tipos de bloques
- **Pasto**: Bloques verdes con textura diferente arriba y a los lados
- **Tierra**: Bloques cafés
- **Piedra**: Bloques grises
- **Tronco**: Maderos con textura diferente en los extremos
- **Hojas**: Transparentes, se ven geniales con el cielo de fondo
- **Agua**: Con reflejos y transparencia, se ve bien realista
- **Lámparas**: Lo mejor - se encienden de noche y realmente alumbran

### Sistema día/noche
- **Día**: Todo se ve normal y brillante
- **Noche**: El cielo se pone oscuro con estrellas y las lámparas se encienden automáticamente
- Las lámparas de verdad iluminan los bloques cercanos, no es solo cambio de textura

### Agua con reflejos
El agua puede reflejar el cielo o hasta otros objetos dependiendo de como la configurés con F6.

### Optimizaciones
El render tiene varias resoluciones para que no se trabe tu computadora:
- Resolución baja: 320x180 (rápido pero pixelado)
- Resolución media: 640x360 (balance decente)

## Estructura del proyecto

```
src/
├── main.rs          - El programa principal
├── camera.rs        - Manejo de la cámara orbital
├── world.rs         - Carga de mundo y tipos de bloques
├── light.rs         - Sistema de iluminación
├── hud.rs           - Interfaz de usuario
├── geometry.rs      - Operaciones geométricas
└── raytracer/       - Todo el ray tracing
    ├── mod.rs       - Estructura principal
    ├── renderer.rs  - Renderizado multihilo
    ├── shade.rs     - Cálculos de iluminación
    ├── sample.rs    - Sampling de texturas
    ├── fog.rs       - Cielo y estrellas
    └── cam.rs       - Matemáticas de cámara
```

## Assets necesarios

En la carpeta `assets/` necesitas estas texturas:
- `grass.png`, `grasstop.png` - Texturas de pasto
- `dirt.png` - Textura de tierra
- `stone.png` - Textura de piedra
- `log_side.png`, `log_top.png` - Texturas de tronco
- `leaves.png` - Textura de hojas
- `water.png` - Textura de agua
- `lamp_off.png`, `lamp_on.png` - Texturas de lámparas

Y en `assets/layers/` los archivos de capas (layer_00.txt hasta layer_08.txt).

## Como modificar el mundo

Los archivos en `assets/layers/` son de texto plano con caracteres que representan bloques:
- `G` = Pasto
- `D` = Tierra  
- `S` = Piedra
- `T` = Tronco
- `L` = Hojas
- `W` = Agua
- `P` = Lámpara
- ` ` (espacio) = Aire

Cada archivo es una cuadrícula de 16x16 caracteres.

## Si algo no funciona

1. Verifica que tengas todas las texturas en la carpeta `assets/`
2. Asegurate que Rust esté bien instalado
3. Si va muy lento, cambia a resolución baja con F1
4. Si se crashea, prueba compilar sin `--release` para ver errores más claros