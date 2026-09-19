/**
 * Aevum Home Page Module
 *
 * Page-specific logic for index.html.
 *
 * Contract:
 *   initHomePage() — async, idempotent.
 *
 * Responsibilities:
 *   - Theme color meta update.
 *   - Three.js bee animation.
 *
 * Does not:
 *   - Load header or footer (js/components.js).
 *   - Handle mobile drawer (js/navigation.js).
 */

const LOG_PREFIX = "[Aevum Home]";

let initialized = false;

function updateThemeColor() {
    const meta = document.querySelector('meta[name="theme-color"]');

    if (!meta) {
        return;
    }

    const isLight =
        document.documentElement.getAttribute("data-theme") === "light";

    meta.setAttribute(
        "content",
        isLight ? "#F0F4FA" : "#061426"
    );
}

function initThreeScene() {

                const container =
                    document.getElementById(
                        "bee-container"
                    );

                if (!container) {
                    return;
                }


                /* ==============================================
                   REDUCED MOTION
                   ============================================== */

                const reducedMotion =
                    window.matchMedia(
                        "(prefers-reduced-motion: reduce)"
                    );

                if (reducedMotion.matches) {
                    return;
                }


                /* ==============================================
                   FALLBACK
                   ============================================== */

                function fallback() {

                    container.innerHTML = "";

                    const element =
                        document.createElement(
                            "div"
                        );

                    element.className =
                        "bee-fallback";

                    element.textContent =
                        "AEVUM";

                    container.appendChild(
                        element
                    );
                }


                /* ==============================================
                   THREE.JS AVAILABILITY
                   ============================================== */

                if (
                    typeof window.THREE ===
                    "undefined"
                ) {
                    console.warn(
                        "[Aevum] Three.js unavailable."
                    );

                    fallback();
                    return;
                }


                const THREE = window.THREE;


                /* ==============================================
                   SCENE
                   ============================================== */

                const scene =
                    new THREE.Scene();


                /* ==============================================
                   CAMERA
                   ============================================== */

                const camera =
                    new THREE.PerspectiveCamera(
                        45,
                        container.clientWidth /
                            container.clientHeight,
                        0.1,
                        1000
                    );

                camera.position.set(
                    0,
                    0,
                    5
                );


                /* ==============================================
                   RENDERER
                   ============================================== */

                let renderer;

                try {

                    renderer =
                        new THREE.WebGLRenderer({
                            antialias: true,
                            alpha: true,
                            powerPreference:
                                "high-performance"
                        });

                } catch (error) {

                    console.warn(
                        "[Aevum] WebGL unavailable.",
                        error
                    );

                    fallback();
                    return;
                }


                renderer.setPixelRatio(
                    Math.min(
                        window.devicePixelRatio ||
                            1,
                        2
                    )
                );

                renderer.setSize(
                    container.clientWidth,
                    container.clientHeight,
                    false
                );

                if (
                    "outputEncoding" in
                    renderer
                ) {
                    renderer.outputEncoding =
                        THREE.sRGBEncoding;
                }

                container.appendChild(
                    renderer.domElement
                );


                /* ==============================================
                   LIGHTING
                   ============================================== */

                const ambient =
                    new THREE.AmbientLight(
                        0x404060,
                        1
                    );

                scene.add(ambient);


                const goldLight =
                    new THREE.PointLight(
                        0xF5C542,
                        1.2,
                        20
                    );

                goldLight.position.set(
                    2,
                    3,
                    5
                );

                scene.add(goldLight);


                const cyanLight =
                    new THREE.PointLight(
                        0x20D9FF,
                        0.6,
                        20
                    );

                cyanLight.position.set(
                    -3,
                    -1,
                    4
                );

                scene.add(cyanLight);


                /* ==============================================
                   AEVUM GROUP
                   ============================================== */

                const group =
                    new THREE.Group();

                scene.add(group);


                /* ==============================================
                   BODY
                   ============================================== */

                const bodyGeometry =
                    new THREE.SphereGeometry(
                        0.5,
                        20,
                        20
                    );

                const bodyMaterial =
                    new THREE.MeshStandardMaterial({
                        color: 0xF5C542,
                        emissive: 0xF5C542,
                        emissiveIntensity: 0.12,
                        metalness: 0.35,
                        roughness: 0.48
                    });

                const body =
                    new THREE.Mesh(
                        bodyGeometry,
                        bodyMaterial
                    );

                body.scale.set(
                    1,
                    0.7,
                    1.2
                );

                group.add(body);


                /* ==============================================
                   STRIPES
                   ============================================== */

                const stripeGeometry =
                    new THREE.BoxGeometry(
                        0.04,
                        0.5,
                        0.5
                    );

                const stripeMaterial =
                    new THREE.MeshStandardMaterial({
                        color: 0x8B5E00,
                        metalness: 0.3,
                        roughness: 0.5
                    });

                for (
                    let i = -2;
                    i <= 2;
                    i++
                ) {

                    const stripe =
                        new THREE.Mesh(
                            stripeGeometry,
                            stripeMaterial
                        );

                    stripe.position.z =
                        i * 0.2;

                    group.add(stripe);
                }


                /* ==============================================
                   WINGS
                   ============================================== */

                const wingGeometry =
                    new THREE.PlaneGeometry(
                        0.7,
                        0.35
                    );

                const wingMaterial =
                    new THREE.MeshStandardMaterial({
                        color: 0x20D9FF,
                        transparent: true,
                        opacity: 0.30,
                        emissive: 0x20D9FF,
                        emissiveIntensity: 0.15,
                        side: THREE.DoubleSide
                    });


                const wingLeft =
                    new THREE.Mesh(
                        wingGeometry,
                        wingMaterial
                    );

                wingLeft.position.set(
                    -0.25,
                    0.2,
                    0
                );

                wingLeft.rotation.x =
                    -0.2;

                wingLeft.rotation.z =
                    -0.5;

                group.add(wingLeft);


                const wingRight =
                    new THREE.Mesh(
                        wingGeometry,
                        wingMaterial
                    );

                wingRight.position.set(
                    0.25,
                    0.2,
                    0
                );

                wingRight.rotation.x =
                    -0.2;

                wingRight.rotation.z =
                    0.5;

                group.add(wingRight);


                /* ==============================================
                   HEAD
                   ============================================== */

                const headGeometry =
                    new THREE.SphereGeometry(
                        0.18,
                        12,
                        12
                    );

                const headMaterial =
                    new THREE.MeshStandardMaterial({
                        color: 0xF5C542,
                        metalness: 0.3,
                        roughness: 0.5
                    });

                const head =
                    new THREE.Mesh(
                        headGeometry,
                        headMaterial
                    );

                head.position.set(
                    0,
                    0.3,
                    0.5
                );

                group.add(head);


                /* ==============================================
                   EYES
                   ============================================== */

                const eyeGeometry =
                    new THREE.SphereGeometry(
                        0.035,
                        8,
                        8
                    );

                const eyeMaterial =
                    new THREE.MeshStandardMaterial({
                        color: 0x061426
                    });


                const eyeLeft =
                    new THREE.Mesh(
                        eyeGeometry,
                        eyeMaterial
                    );

                eyeLeft.position.set(
                    -0.08,
                    0.35,
                    0.6
                );

                group.add(eyeLeft);


                const eyeRight =
                    new THREE.Mesh(
                        eyeGeometry,
                        eyeMaterial
                    );

                eyeRight.position.set(
                    0.08,
                    0.35,
                    0.6
                );

                group.add(eyeRight);


                /* ==============================================
                   COIN
                   ============================================== */

                const coinGroup =
                    new THREE.Group();

                coinGroup.position.set(
                    0,
                    -0.55,
                    0.15
                );

                group.add(coinGroup);


                const coinGeometry =
                    new THREE.CylinderGeometry(
                        0.35,
                        0.35,
                        0.05,
                        32
                    );

                const coinMaterial =
                    new THREE.MeshStandardMaterial({
                        color: 0xF5C542,
                        metalness: 0.8,
                        roughness: 0.2,
                        emissive: 0xF5C542,
                        emissiveIntensity: 0.08
                    });

                const coin =
                    new THREE.Mesh(
                        coinGeometry,
                        coinMaterial
                    );

                coin.rotation.x =
                    Math.PI / 2;

                coinGroup.add(coin);


                /* ==============================================
                   COIN LETTERS
                   ============================================== */

                function createLetterTexture(
                    letter
                ) {

                    const canvas =
                        document.createElement(
                            "canvas"
                        );

                    canvas.width = 128;
                    canvas.height = 128;

                    const context =
                        canvas.getContext(
                            "2d"
                        );

                    if (!context) {
                        return null;
                    }

                    context.fillStyle =
                        "#F5C542";

                    context.fillRect(
                        0,
                        0,
                        128,
                        128
                    );

                    context.fillStyle =
                        "#061426";

                    context.font =
                        "bold 64px Arial";

                    context.textAlign =
                        "center";

                    context.textBaseline =
                        "middle";

                    context.fillText(
                        letter,
                        64,
                        66
                    );

                    const texture =
                        new THREE.CanvasTexture(
                            canvas
                        );

                    texture.needsUpdate =
                        true;

                    return texture;
                }


                const textureA =
                    createLetterTexture(
                        "A"
                    );

                const textureB =
                    createLetterTexture(
                        "B"
                    );


                if (
                    textureA &&
                    textureB
                ) {

                    const letterGeometry =
                        new THREE.PlaneGeometry(
                            0.28,
                            0.28
                        );


                    const letterMaterialA =
                        new THREE.MeshStandardMaterial({
                            map: textureA,
                            transparent: true,
                            side: THREE.DoubleSide
                        });

                    const letterA =
                        new THREE.Mesh(
                            letterGeometry,
                            letterMaterialA
                        );

                    letterA.position.z =
                        0.035;

                    coinGroup.add(
                        letterA
                    );


                    const letterMaterialB =
                        new THREE.MeshStandardMaterial({
                            map: textureB,
                            transparent: true,
                            side: THREE.DoubleSide
                        });

                    const letterB =
                        new THREE.Mesh(
                            letterGeometry.clone(),
                            letterMaterialB
                        );

                    letterB.position.z =
                        -0.035;

                    letterB.rotation.y =
                        Math.PI;

                    coinGroup.add(
                        letterB
                    );
                }


                /* ==============================================
                   POINTER
                   ============================================== */

                let targetMouseX = 0;
                let targetMouseY = 0;


                function pointerMove(
                    event
                ) {

                    const rect =
                        container.getBoundingClientRect();

                    if (
                        !rect.width ||
                        !rect.height
                    ) {
                        return;
                    }

                    targetMouseX =
                        (
                            (
                                event.clientX -
                                rect.left
                            ) /
                            rect.width
                        ) * 2 - 1;

                    targetMouseY =
                        -(
                            (
                                event.clientY -
                                rect.top
                            ) /
                            rect.height
                        ) * 2 + 1;
                }


                function pointerLeave() {

                    targetMouseX = 0;
                    targetMouseY = 0;
                }


                container.addEventListener(
                    "pointermove",
                    pointerMove,
                    { passive: true }
                );

                container.addEventListener(
                    "pointerleave",
                    pointerLeave,
                    { passive: true }
                );


                /* ==============================================
                   RESIZE
                   ============================================== */

                function resize() {

                    const width =
                        container.clientWidth;

                    const height =
                        container.clientHeight;

                    if (
                        !width ||
                        !height
                    ) {
                        return;
                    }

                    camera.aspect =
                        width / height;

                    camera.updateProjectionMatrix();

                    renderer.setSize(
                        width,
                        height,
                        false
                    );
                }


                resize();


                const resizeObserver =
                    new ResizeObserver(
                        resize
                    );

                resizeObserver.observe(
                    container
                );


                /* ==============================================
                   PAGE VISIBILITY
                   ============================================== */

                let paused =
                    document.hidden;


                function handleVisibility() {
                    paused =
                        document.hidden;
                }


                document.addEventListener(
                    "visibilitychange",
                    handleVisibility
                );


                /* ==============================================
                   ANIMATION
                   ============================================== */

                let animationFrame =
                    null;

                let time = 0;

                let lastFrame =
                    performance.now();


                function animate(
                    now
                ) {

                    animationFrame =
                        requestAnimationFrame(
                            animate
                        );

                    if (paused) {
                        lastFrame = now;
                        return;
                    }


                    const delta =
                        Math.min(
                            (
                                now -
                                lastFrame
                            ) / 1000,
                            0.05
                        );

                    lastFrame = now;

                    time += delta;


                    /* FLOAT */

                    group.position.y =
                        Math.sin(
                            time * 0.8
                        ) * 0.06;


                    /* COIN */

                    coinGroup.rotation.y +=
                        delta * 0.9;


                    /* POINTER */

                    group.rotation.x +=
                        (
                            targetMouseY *
                            0.05 -
                            group.rotation.x
                        ) * 0.02;

                    group.rotation.y +=
                        (
                            targetMouseX *
                            0.05 -
                            group.rotation.y
                        ) * 0.02;


                    /* WINGS */

                    const wingAngle =
                        Math.sin(
                            time * 10
                        ) * 0.35;

                    wingLeft.rotation.y =
                        -0.5 +
                        wingAngle * 0.5;

                    wingRight.rotation.y =
                        0.5 -
                        wingAngle * 0.5;


                    renderer.render(
                        scene,
                        camera
                    );
                }


                animationFrame =
                    requestAnimationFrame(
                        animate
                    );


                /* ==============================================
                   CLEANUP
                   ============================================== */

                function cleanup() {

                    if (
                        animationFrame !== null
                    ) {
                        cancelAnimationFrame(
                            animationFrame
                        );
                    }

                    resizeObserver.disconnect();

                    document.removeEventListener(
                        "visibilitychange",
                        handleVisibility
                    );

                    container.removeEventListener(
                        "pointermove",
                        pointerMove
                    );

                    container.removeEventListener(
                        "pointerleave",
                        pointerLeave
                    );


                    scene.traverse(
                        (object) => {

                            if (
                                !object.isMesh
                            ) {
                                return;
                            }

                            object.geometry?.dispose();

                            if (
                                Array.isArray(
                                    object.material
                                )
                            ) {

                                object.material.forEach(
                                    material => {
                                        material.dispose();
                                    }
                                );

                            } else {

                                object.material?.dispose();
                            }
                        }
                    );


                    textureA?.dispose();
                    textureB?.dispose();

                    renderer.dispose();
                }


                window.addEventListener(
                    "pagehide",
                    cleanup,
                    { once: true }
                );
}

export async function initHomePage() {
    if (initialized) {
        return;
    }

    initialized = true;

    updateThemeColor();

    document.addEventListener(
        "aevum:theme-changed",
        updateThemeColor
    );

    initThreeScene();
}
