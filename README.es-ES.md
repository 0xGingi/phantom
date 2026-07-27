# phantom

phantom es un editor de texto para linux basado en terminal, ligero y escrito en Rust. Combina la simplicidad de un editor de texto básico con algunas características potentes inspiradas en Vim.

![image](https://github.com/user-attachments/assets/3f71a03d-68c3-4be0-b199-9e40a799d577)


## Características

- Interfaz simple e intuitiva
- Edición modal estilo Vim (modos Normal, Insertar, Visual y Comando)
- Resaltado de sintaxis
- Integración con el portapapeles del sistema
- Personalizable (Actualmente atajos de teclado y colores)
- Navegación de directorios (Barra lateral)
- Agentes de codificación CLI (Barra lateral)
- Menú de salida de depuración (Debug Output)
- Búsqueda en el archivo
- Deshacer (Undo) y Rehacer (Redo)
- Pestañas
- Minimapa
- Estado de Git
- Auto-indentación inteligente
- Búsqueda mejorada con regex y opciones insensibles a mayúsculas/minúsculas
- Deshacer/rehacer inteligente con agrupación de operaciones
- Temas intercambiables

## Estado Multiplataforma

- Linux: 100%
- MacOS: No planeado
- Windows: No planeado

## Instalación

### Arch User Repository

#### Binario

[![binary](https://img.shields.io/aur/version/phantom-editor-bin)](https://aur.archlinux.org/packages/phantom-editor-bin)

#### Git

[![git](https://img.shields.io/aur/version/phantom-editor-git)](https://aur.archlinux.org/packages/phantom-editor-git)

### Versión Binaria (Release)

Descarga el ejecutable más reciente de phantom desde [releases](https://github.com/0xGingi/phantom/releases)

Coloca el ejecutable en `/usr/bin` (o en cualquier carpeta dentro de tu path)

### Compilar desde el código fuente

1. Asegúrate de tener Rust y Cargo instalados en tu sistema. Si no es así, instálalos desde [https://www.rust-lang.org/](https://www.rust-lang.org/).

2. Clona este repositorio:
   ```
   git clone https://github.com/0xGingi/phantom.git
   ```

3. Navega al directorio del proyecto:
   ```
   cd phantom
   ```

4. Compila el proyecto:
   ```
   cargo build --release
   ```

5. El ejecutable se creará en el directorio `target/release`.

## Uso

Para iniciar phantom:
```
phantom
phantom archivo.txt
phantom ~/Proyecto
```

Si se proporciona un nombre de archivo, phantom intentará abrir ese archivo. De lo contrario, iniciará con un documento en blanco.
Si se proporciona un directorio, phantom entrará en el modo de navegación de directorios.

## Atajos de Teclado y Comandos Predeterminados

### Ubicaciones del archivo de configuración

- Linux: `~/.config/phantom`

### Globales

- `Ctrl+Q`: Salir del editor
- `?`: Mostrar ayuda de atajos de teclado

### Modo Normal

- `i` o `Insert` : Entrar en modo Insertar
- `a`: Entrar en modo Insertar después del cursor
- `o`: Insertar una nueva línea debajo y entrar en modo Insertar
- `O`: Insertar una nueva línea arriba y entrar en modo Insertar
- `dd`: Eliminar la línea actual
- `yy`: Arrancar (copiar) la línea actual
- `p`: Pegar después de la línea actual
- `Ctrl+Y`: Copiar la línea actual al portapapeles del sistema
- `Ctrl+P`: Pegar desde el portapapeles del sistema debajo de la línea actual
- `v`: Entrar en modo Visual
- Teclas de flecha: Mover el cursor
- `Home`: Mover al inicio de la línea
- `End`: Mover al final de la línea
- `Delete`: Eliminar el carácter bajo el cursor
- `:`: Entrar en modo Comando
- `Ctrl+B`: Alternar visibilidad del menú de depuración
- `Ctrl+E`: Entrar en modo de navegación de directorios
- `Ctrl+G`: Abrir la barra lateral de agentes
- `/`: Entrar en modo Búsqueda
- `n`: Ir al siguiente resultado de búsqueda
- `N`: Ir al resultado de búsqueda anterior
- `PageUp`: Desplazarse hacia arriba una página
- `PageDown`: Desplazarse hacia abajo una página
- `Ctrl+U`: Deshacer
- `Ctrl+R`: Rehacer
- `Ctrl+T`: Nueva Pestaña
- `Ctrl+W`: Cerrar Pestaña
- `F1`-`F9`: Cambiar a la Pestaña 1-9
- `Tab`: Alternar entre pestañas
- `Ctrl+M`: Alternar Minimapa
- `Ctrl+L`: Alternar modo de números de línea (Apagado/Absoluto/Relativo/Híbrido)
- `Ctrl+I`: Alternar auto-indentación
- `Ctrl+J`: Alternar ajuste de línea (word wrap)
- `Shift+T`: Ciclar entre temas de color
- `?`: Mostrar ayuda de atajos de teclado
- `Clic del Mouse`: Mover cursor a la posición clickeada
- `Rueda del Mouse`: Desplazarse arriba/abajo (3 líneas)
- `Shift+Rueda del Mouse`: Desplazarse izquierda/derecha (5 columnas)

### Modo Insertar

- `Esc`: Volver al modo Normal
- `Enter`: Insertar una nueva línea con auto-indentación inteligente
- `Backspace`: Eliminar el carácter antes del cursor
- Cualquier tecla de carácter: Insertar el carácter en la posición del cursor

### Modo Visual

- `Esc`: Volver al modo Normal
- `y`: Copiar el texto seleccionado al portapapeles del sistema
- Teclas de flecha: Extender selección

### Modo Comando

- `:w`: Guardar el archivo actual
- `:w nombre_archivo`: Guardar el archivo actual como 'nombre_archivo'
- `:q`: Salir del editor
- `:wq`: Guardar y salir
- `:e nombre_archivo`: Abrir 'nombre_archivo' para editar
- `:agents`: Alternar barra lateral de agentes
- `:agent <comando>`: Lanzar un comando de agente personalizado en la barra lateral

### Modo Búsqueda

- `Enter`: Realizar búsqueda y volver al modo Normal
- `Esc`: Cancelar búsqueda y volver al modo Normal
- `Alt+I`: Alternar búsqueda sensible a mayúsculas/minúsculas
- `Alt+R`: Alternar modo de búsqueda regex
- Cualquier carácter: Búsqueda en vivo mientras escribes

## Características Mejoradas

### Auto-indentación Inteligente
- Indenta automáticamente las nuevas líneas basándose en la línea anterior
- Reconoce estructuras comunes de programación (if, for, while, llaves)
- Soporta tanto tabulaciones como espacios con tamaño de sangría configurable
- Alternar con `Ctrl+I`

### Búsqueda Avanzada
- **Búsqueda en vivo**: Los resultados se actualizan mientras escribes
- **Sensibilidad a mayúsculas**: Alternar con `Alt+I` en modo de búsqueda
- **Soporte Regex**: Alternar con `Alt+R` en modo de búsqueda
- **Coincidencias múltiples**: Encuentra todas las apariciones en el archivo
- Usa `n` y `N` para navegar entre los resultados de búsqueda

### Deshacer/Rehacer Inteligente
- Agrupa inserciones/eliminaciones de caracteres consecutivas en operaciones lógicas
- Agrupación basada en tiempo (operaciones dentro de 1 segundo)
- Comportamiento de deshacer más intuitivo para un mejor flujo de edición

### Soporte de Mouse
- **Clic para posicionar**: Haz clic en cualquier parte del editor para mover el cursor
- **Desplazamiento Vertical**: La rueda del mouse se desplaza arriba/abajo
- **Desplazamiento Horizontal**: Shift+rueda del mouse se desplaza izquierda/derecha
- **Selección de Texto**: Arrastra para seleccionar texto, clic derecho para copiar

### Números de Línea
- **Absoluto**: Muestra los números de línea reales (1, 2, 3...)
- **Relativo**: Muestra la distancia desde la línea actual
- **Híbrido**: Muestra el número de línea actual + distancias relativas
- Alternar modos con `Ctrl+L`

### Temas de Color
- **One Dark** (predeterminado): Tema oscuro moderno inspirado en Atom/VS Code
- **Dracula**: Popular tema con acentos púrpura y rosa
- **Solarized Dark**: Esquema de colores diseñado científicamente, agradable a la vista
- **Nord**: Tema azul frío inspirado en el Ártico
- **Monokai**: Colores cálidos clásicos, popular en Sublime Text
- **Gruvbox**: Colores retro con tonos cálidos y terrosos
- Ciclar entre temas con `Shift+T`

## Salida de Depuración (Debug Output)

phantom incluye un área de salida de depuración que muestra información sobre las pulsaciones de teclas, la posición del cursor y los resultados de operaciones como el guardado de archivos.
