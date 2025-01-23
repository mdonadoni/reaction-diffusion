# reaction-diffusion

GPU-accelerated simulation of a reaction-diffusion system in Rust, using [wgpu](https://wgpu.rs/).

The simulation can be run natively, or it can be compiled to WebAssembly and executed in a browser supporting WebGPU.

👉 [Click here](https://mdonadoni.github.io/reaction-diffusion/) to run the simulation in your browser!

## Gray Scott model

Reaction-diffusion systems model the concentration in space and time of chemical substances. As the name implies, the reagents can _diffuse_ through space and _react_ with each other.

The Gray Scott model is a specific reaction-diffusion system that simulates the behaviour of the following chemical reactions:

```math
A + 2B \rightarrow 3B \\
B \rightarrow C
```

The concentration of reagents $A$ and $B$ is known at every point in space, and is represented by the functions $a$ and $b$. The substance $C$ is an inert product that no longer reacts.

The system is described by the following equations:

```math
\frac{ \partial a }{ \partial t } = D_a \nabla^2 a - a b^2 + f (1 - a) \\
\frac{ \partial b }{ \partial t } = D_b \nabla^2 b + a b^2 - (k + f) b
```

where:

- $D_a$ and $D_b$ are the diffusion rates
- $f$ is the feed rate of substance $A$
- $k$ is the kill rate of substance $B$

In particular:

- The term $D_x \nabla^2 x$ describes how the substances diffuse
- The terms $-ab^2$ and $ab^2$ describe the decrement of substance $A$ and the increment of substance $B$ due to the reaction $A + 2B \rightarrow 3B$
- The term $f(1-a)$ describes the replenishment of substance $A$, as otherwise it would eventually all be consumed by the first reaction
- The term $(k+f)b$ describes the diminishment of substance $B$ due to the reaction $B \rightarrow C$

This system of equations is then discretised so that it can be simulated on a grid of cells, where each cell has specific concentrations of $A$ and $B$.

### References

- [Reaction-Diffusion Tutorial](https://www.karlsims.com/rd.html)
- [Gray Scott Model of Reaction Diffusion](https://groups.csail.mit.edu/mac/projects/amorphous/GrayScott/)
- [Reaction-Diffusion by the Gray-Scott Model: Pearson's Parametrization](http://www.mrob.com/pub/comp/xmorphia/index.html)
