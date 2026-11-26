# Ideas de Atoll

Acá viven las ideas todavía sin decidir o sin fecha. Lo que ya está priorizado vive en `docs/ROADMAP.md`;
cuando una ficha pasa a ser trabajo concreto, se mueve al roadmap y se deja el enlace acá.

Cada ficha tiene: **veredicto** (Plugin, Core, Later, Descartar), **qué es**, **por qué encaja (o no)** con las
reglas innegociables, **riesgos y preguntas abiertas**, **dependencias** y **próximo paso**. Los veredictos los
fija el director del proyecto; los comentarios marcados como *Discusión* son observaciones para confirmarlos o
discutirlos, no decisiones.

## Posicionamiento propuesto

**Liviana, transparente, para gente técnica.** Liviana: menos de 20 MB de RAM y 0 % de CPU en reposo, medido y
reproducible. Transparente: código abierto (Apache-2.0), mediciones publicadas con el comando para repetirlas, y
sin afirmar nada que no esté medido o funcionando. Para gente técnica: configuración en un archivo de texto, sin
cuentas ni nube, y plugins como procesos separados que se pueden leer y auditar.

## Orden propuesto

1. Spike de viabilidad de Atoll Guard.
2. Ítem 15 (protocolo IPC y host de plugins).
3. Atoll Guard (plugin).
4. Ajustes (`atoll-settings`) con ícono en la bandeja e inicio con Windows.
5. Monitoreo de agentes de IA (plugin).
6. Música (plugin).
7. grandMA (plugin de nicho).
8. Publicación en winget.
9. v1.0.

*Discusión:* este orden difiere del `ROADMAP.md` actual en tres puntos: (a) el ítem 16 (idiomas es/en) no
aparece (se asume que sigue en v0.3, junto al 15, o se reubica); (b) Guard y el monitoreo de IAs pasan por
delante de `atoll-settings` y de la música, que en el roadmap van antes; y (c) winget queda justo antes de v1.0
en lugar de dentro de "Distribución". Mientras el director no lo confirme, el `ROADMAP.md` no se modifica.

---

## Atoll Guard: indicadores de micrófono, cámara y ubicación

- **Veredicto:** PLUGIN, primero. Pendiente un spike de viabilidad.
- **Qué es:** mostrar en la isla cuándo una aplicación está usando el micrófono, la cámara o la ubicación, con el
  nombre de la app y un historial de usos.
- **Por qué encaja:** es información útil para el público técnico y no necesita nube ni claves. Como plugin, no
  suma peso al Core cuando no está instalado.
- **Riesgos y preguntas abiertas (a resolver en el spike):**
  - ¿De dónde sale el dato en Windows de forma confiable y **sin polling**? Hipótesis a verificar, no
    afirmada: la base de consentimientos de privacidad del sistema (clave `CapabilityAccessManager\ConsentStore`
    del registro) registra, por capacidad y por app, marcas de último uso; si es así, podría observarse con una
    notificación de cambios del registro (orientada a eventos) en lugar de consultas periódicas.
  - Cobertura real: ¿distingue aplicaciones de escritorio clásicas de las empaquetadas? ¿Cubre el uso por
    servicios o procesos sin ventana? ¿Hay falsos negativos? El texto público no puede prometer "detecta todo".
  - Es un **indicador**, no una barrera: no bloquea ni garantiza que nada se use sin aviso. Hay que decirlo así.
  - Privacidad del historial: dónde se guarda, cuánto tiempo, cómo se borra y que sea solo local y legible por el
    usuario.
  - Costo de RAM y CPU del plugin, medido contra el presupuesto.
- **Dependencias:** ítem 15 (host de plugins y árbol de widgets) para el plugin definitivo; el spike no depende
  de él.
- **Próximo paso:** spike de viabilidad con un programa de prueba aparte (sin tocar el Core): fuente del dato,
  eventos vs. polling, cobertura, consumo. Resultado escrito en esta ficha antes de decidir el diseño.

## Monitoreo de agentes de IA

- **Veredicto:** PLUGIN, segundo.
- **Qué es:** mostrar el estado de agentes de IA que corren en la máquina (trabajando, esperando, terminó),
  alimentado por un comando local `atoll-notify` que el agente o un script invoca, sin claves ni nube.
- **Por qué encaja:** el estado entra por un canal local y el plugin solo dibuja un árbol de widgets. No hay
  integraciones con servicios externos.
- **Riesgos y preguntas abiertas:**
  - Seguridad del canal local: cualquier proceso del usuario podría mandar estados falsos. La tubería de IPC
    debe restringir quién puede conectarse (solo el mismo usuario), tal como ya está registrado en "Deuda y
    seguimiento" (d) del roadmap.
  - Definir el formato del mensaje de `atoll-notify` (agente, estado, texto corto) y su versionado junto con el
    protocolo del ítem 15.
  - Cómo se enteran los agentes de que deben llamar a `atoll-notify` (documentación y ejemplos por herramienta),
    sin que Atoll prometa compatibilidad con productos concretos que no haya probado.
  - Qué pasa si un agente muere sin avisar: política de vencimiento del estado sin usar timers extra en el Core.
- **Dependencias:** ítem 15 (IPC y host de plugins).
- **Próximo paso:** esperar al ítem 15; mientras tanto, redactar el contrato de `atoll-notify`.

## Integración con grandMA2 y grandMA3

- **Veredicto:** PLUGIN de nicho, después del host de plugins.
- **Qué es:** mostrar y/o controlar desde la isla información de consolas de iluminación grandMA2 y grandMA3.
- **Por qué encaja (y por qué es de nicho):** el público es reducido, así que solo tiene sentido como plugin
  opcional, aislado, sin costo para quien no lo usa.
- **Riesgos y preguntas abiertas:**
  - **Verificar el protocolo con la documentación oficial de MA Lighting antes de diseñar nada.** No se asume
    qué mecanismos de control remoto ofrece cada versión ni sus limitaciones.
  - Requiere conectarse a la red local: hay que definir qué se configura, dónde se guardan datos de conexión y
    qué pasa cuando la consola no está disponible (sin reintentos agresivos ni polling).
  - Alcance: qué se muestra (estado) frente a qué se controla (acciones), porque controlar luces en vivo exige
    cuidado.
  - Marcas y nombres: usar los nombres de producto de forma descriptiva, sin sugerir afiliación con MA Lighting.
- **Dependencias:** ítem 15.
- **Próximo paso:** leer la documentación oficial y escribir en esta ficha qué permite el protocolo; recién
  después estimar el alcance.

## Conexión desde el celular

- **Veredicto:** LATER, después de v1.0.
- **Qué es:** ver o controlar Atoll desde un teléfono.
- **Por qué no ahora:** exige escuchar en la red, emparejar dispositivos, cifrar el tráfico y mantener una app
  móvil. Choca con la política actual de IPC solo local y suma superficie de ataque.
- **Riesgos y preguntas abiertas:**
  - Escuchar en la red por defecto sería una regresión de seguridad: tendría que ser opt-in, con emparejamiento
    explícito y cifrado.
  - Una app móvil es un producto aparte (plataformas, tiendas, mantenimiento).
  - Impacto en RAM y CPU en reposo del Core o del plugin que escuche.
- **Dependencias:** ítem 15, modelo de permisos de plugins y v1.0 publicada.
- **Próximo paso:** ninguno hasta después de v1.0.

## Navegador propio en Rust

- **Veredicto:** DESCARTAR de Atoll. Si se quiere hacer, es un proyecto aparte.
- **Qué es:** un navegador web escrito en Rust.
- **Por qué se descarta:** contradice las reglas innegociables. Un motor de navegador rompe el presupuesto de
  menos de 20 MB de RAM y la regla de Core sin webview ni frameworks de UI, y su alcance es de otro orden de
  magnitud que una isla liviana.
- **Próximo paso:** ninguno en este repositorio.

## Arrastrar archivos a la isla con acciones

- **Veredicto:** LATER (bandeja de archivos).
- **Qué es:** soltar archivos sobre la isla y elegir una acción (por ejemplo, guardarlos temporalmente en una
  bandeja de archivos).
- **Por qué encaja:** es la base natural del plugin "bandeja de archivos" ya listado en el roadmap.
- **Riesgos y preguntas abiertas:**
  - Habría que verificar cómo recibe arrastres la ventana de la isla (es una ventana superpuesta que no toma
    el foco) y qué mecanismo del sistema hace falta, y si eso suma dependencias o timers.
  - Qué hace con los archivos: copiar, mover o solo referenciar; límites de tamaño y limpieza.
  - Los plugins no abren ventanas propias: la zona de soltar y las acciones deben expresarse con el árbol de
    widgets que dibuja el Core.
- **Dependencias:** ítem 15 y plugin de bandeja de archivos.
- **Próximo paso:** retomarlo cuando se diseñe la bandeja de archivos.
