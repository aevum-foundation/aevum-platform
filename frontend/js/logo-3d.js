// === AEVUM 3D LOGO ===
(function() {
    // Проверяем, есть ли контейнер
    const container = document.getElementById('logo-3d-container');
    if (!container) return;

    // === SCENE ===
    const scene = new THREE.Scene();
    scene.background = null; // прозрачный фон

    const camera = new THREE.PerspectiveCamera(45, container.clientWidth / container.clientHeight, 0.1, 1000);
    camera.position.set(0, 0, 6);

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setSize(container.clientWidth, container.clientHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    container.appendChild(renderer.domElement);

    // === LIGHTS ===
    scene.add(new THREE.AmbientLight(0x404060));
    const light1 = new THREE.PointLight(0xD4AF37, 1.2, 20);
    light1.position.set(2, 3, 5);
    scene.add(light1);
    const light2 = new THREE.PointLight(0x10B981, 0.6, 20);
    light2.position.set(-3, -1, 4);
    scene.add(light2);

    // === NODES ===
    const nodeGroup = new THREE.Group();
    const nodeData = [];
    const numNodes = 80;
    const spread = 2.8;
    const positions = [];

    for (let i = 0; i < numNodes * 2; i++) {
        let x, y, z;
        let inside = false;
        let attempts = 0;
        while (!inside && attempts < 50) {
            attempts++;
            x = (Math.random() - 0.5) * spread * 2;
            y = (Math.random() - 0.5) * spread * 2;
            z = (Math.random() - 0.5) * 1.2;
            const absX = Math.abs(x);
            const leftEdge = 1.2 - (y + 1.2) * 1.1;
            const rightEdge = -1.2 + (y + 1.2) * 1.1;
            const barY = y > -0.25 && y < 0.1;
            const inLeft = x > -0.15 && x < leftEdge && y < 1.0;
            const inRight = x < 0.15 && x > rightEdge && y < 1.0;
            const inBar = barY && absX < 1.0;
            if (inLeft || inRight || inBar) inside = true;
        }
        if (inside) positions.push({ x, y, z });
        if (positions.length >= numNodes) break;
    }

    while (positions.length < 60) {
        const x = (Math.random() - 0.5) * spread * 2;
        const y = (Math.random() - 0.5) * spread * 2;
        const z = (Math.random() - 0.5) * 1.2;
        const absX = Math.abs(x);
        const leftEdge = 1.2 - (y + 1.2) * 1.1;
        const rightEdge = -1.2 + (y + 1.2) * 1.1;
        const barY = y > -0.25 && y < 0.1;
        const inLeft = x > -0.15 && x < leftEdge && y < 1.0;
        const inRight = x < 0.15 && x > rightEdge && y < 1.0;
        const inBar = barY && absX < 1.0;
        if (inLeft || inRight || inBar) positions.push({ x, y, z });
    }

    positions.forEach((pos) => {
        const isGold = Math.random() > 0.35;
        const color = isGold ? 0xD4AF37 : 0x10B981;
        const emissive = isGold ? 0xD4AF37 : 0x10B981;
        const geom = new THREE.SphereGeometry(0.065, 6, 6);
        const mat = new THREE.MeshStandardMaterial({ color, emissive, emissiveIntensity: 0.3, roughness: 0.3, metalness: 0.7 });
        const mesh = new THREE.Mesh(geom, mat);
        mesh.position.set(pos.x, pos.y, pos.z);
        const scale = 0.6 + Math.random() * 0.5;
        mesh.scale.set(scale, scale, scale);
        nodeGroup.add(mesh);
        nodeData.push({ mesh, basePos: { x: pos.x, y: pos.y, z: pos.z }, speed: 0.15 + Math.random() * 0.25, phase: Math.random() * Math.PI * 2 });
    });

    scene.add(nodeGroup);

    // === LINES ===
    const lineMat = new THREE.LineBasicMaterial({ color: 0xD4AF37, transparent: true, opacity: 0.06 });
    const linePositions = [];
    const threshold = 0.9;
    for (let i = 0; i < nodeData.length; i++) {
        for (let j = i + 1; j < nodeData.length; j++) {
            const dx = nodeData[i].mesh.position.x - nodeData[j].mesh.position.x;
            const dy = nodeData[i].mesh.position.y - nodeData[j].mesh.position.y;
            const dz = nodeData[i].mesh.position.z - nodeData[j].mesh.position.z;
            if (Math.sqrt(dx*dx + dy*dy + dz*dz) < threshold) {
                linePositions.push(nodeData[i].mesh.position.x, nodeData[i].mesh.position.y, nodeData[i].mesh.position.z);
                linePositions.push(nodeData[j].mesh.position.x, nodeData[j].mesh.position.y, nodeData[j].mesh.position.z);
            }
        }
    }
    const lineGeom = new THREE.BufferGeometry();
    lineGeom.setAttribute('position', new THREE.Float32BufferAttribute(linePositions, 3));
    const lines = new THREE.LineSegments(lineGeom, lineMat);
    scene.add(lines);

    // === GLOW ===
    const glowGeom = new THREE.SphereGeometry(1.5, 24, 24);
    const glowMat = new THREE.MeshBasicMaterial({ color: 0xD4AF37, transparent: true, opacity: 0.025, wireframe: true });
    const glow = new THREE.Mesh(glowGeom, glowMat);
    scene.add(glow);

    // === MOUSE ===
    let mx = 0, my = 0;
    container.addEventListener('mousemove', (e) => {
        const rect = container.getBoundingClientRect();
        mx = ((e.clientX - rect.left) / rect.width) * 2 - 1;
        my = -((e.clientY - rect.top) / rect.height) * 2 + 1;
    });
    container.addEventListener('mouseleave', () => { mx = 0; my = 0; });

    // === RESIZE ===
    const resize = () => {
        const w = container.clientWidth;
        const h = container.clientHeight;
        camera.aspect = w / h;
        camera.updateProjectionMatrix();
        renderer.setSize(w, h);
    };
    window.addEventListener('resize', resize);

    // === ANIMATE ===
    let t = 0;
    function animate() {
        requestAnimationFrame(animate);
        t += 0.008;
        nodeGroup.rotation.x += (my * 0.08 - nodeGroup.rotation.x) * 0.02;
        nodeGroup.rotation.y += (mx * 0.08 - nodeGroup.rotation.y) * 0.02;
        nodeData.forEach((d) => {
            d.mesh.position.x = d.basePos.x + Math.cos(t * d.speed * 0.7 + d.phase) * 0.015;
            d.mesh.position.y = d.basePos.y + Math.sin(t * d.speed + d.phase) * 0.025;
            d.mesh.position.z = d.basePos.z + Math.sin(t * d.speed * 0.5 + d.phase) * 0.02;
        });
        const lp = [];
        for (let i = 0; i < nodeData.length; i++) {
            for (let j = i + 1; j < nodeData.length; j++) {
                const dx = nodeData[i].mesh.position.x - nodeData[j].mesh.position.x;
                const dy = nodeData[i].mesh.position.y - nodeData[j].mesh.position.y;
                const dz = nodeData[i].mesh.position.z - nodeData[j].mesh.position.z;
                if (Math.sqrt(dx*dx + dy*dy + dz*dz) < threshold) {
                    lp.push(nodeData[i].mesh.position.x, nodeData[i].mesh.position.y, nodeData[i].mesh.position.z);
                    lp.push(nodeData[j].mesh.position.x, nodeData[j].mesh.position.y, nodeData[j].mesh.position.z);
                }
            }
        }
        lines.geometry.setAttribute('position', new THREE.Float32BufferAttribute(lp, 3));
        lines.geometry.attributes.position.needsUpdate = true;
        glow.scale.setScalar(1 + Math.sin(t * 0.6) * 0.02);
        glow.material.opacity = 0.025 + Math.sin(t * 0.6) * 0.008;
        renderer.render(scene, camera);
    }
    animate();
})();
