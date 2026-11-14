// Cena 3D do livro. Carregada só via import() dinâmico para manter `three` fora do chunk inicial.
import * as THREE from 'three';
import { clampDpr } from './support.ts';

export interface HeroScene {
	resize(width: number, height: number): void;
	setActive(active: boolean): void;
	dispose(): void;
}

const INK = 0x111111;
const PAPER = 0xf4f1ea;

export function createHeroScene(canvas: HTMLCanvasElement): HeroScene {
	const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
	renderer.setPixelRatio(clampDpr(window.devicePixelRatio));

	const scene = new THREE.Scene();
	const camera = new THREE.PerspectiveCamera(35, 1, 0.1, 100);
	camera.position.set(0, 0.6, 6);
	camera.lookAt(0, 0, 0);

	const book = new THREE.Group();
	const paperMat = new THREE.MeshBasicMaterial({ color: PAPER });
	const inkMat = new THREE.MeshBasicMaterial({ color: INK });
	const lineMat = new THREE.LineBasicMaterial({ color: INK });
	const geometries: THREE.BufferGeometry[] = [];
	const add = (geo: THREE.BufferGeometry, mat: THREE.Material, x: number) => {
		const edges = new THREE.EdgesGeometry(geo);
		geometries.push(geo, edges);
		const mesh = new THREE.Mesh(geo, mat);
		const line = new THREE.LineSegments(edges, lineMat);
		mesh.position.x = x;
		line.position.x = x;
		return [mesh, line];
	};
	const front = add(new THREE.BoxGeometry(2, 2.8, 0.08), inkMat, 0);
	front.forEach((o) => (o.position.z = 0.25));
	const back = add(new THREE.BoxGeometry(2, 2.8, 0.08), inkMat, 0);
	back.forEach((o) => (o.position.z = -0.25));
	const pages = add(new THREE.BoxGeometry(1.92, 2.7, 0.42), paperMat, 0.03);
	const spine = add(new THREE.BoxGeometry(0.08, 2.8, 0.58), inkMat, -1);
	book.add(...front, ...back, ...pages, ...spine);
	book.rotation.set(0.15, -0.6, 0.05);
	scene.add(book);

	let frame = 0;
	let active = false;
	let last = 0;

	const draw = () => renderer.render(scene, camera);
	const tick = (t: number) => {
		frame = 0;
		if (!active) return;
		const dt = last ? Math.min(t - last, 100) : 16;
		last = t;
		book.rotation.y += dt * 0.0003;
		draw();
		frame = requestAnimationFrame(tick);
	};
	const setActive = (next: boolean) => {
		active = next;
		if (active && !frame) {
			last = 0;
			frame = requestAnimationFrame(tick);
		} else if (!active && frame) {
			cancelAnimationFrame(frame);
			frame = 0;
		}
	};

	return {
		resize(width, height) {
			if (width <= 0 || height <= 0) return;
			renderer.setSize(width, height, false);
			camera.aspect = width / height;
			camera.updateProjectionMatrix();
			draw(); // render on demand
		},
		setActive,
		dispose() {
			setActive(false);
			geometries.forEach((g) => g.dispose());
			paperMat.dispose();
			inkMat.dispose();
			lineMat.dispose();
			renderer.dispose();
		}
	};
}
