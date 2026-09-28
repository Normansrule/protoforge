# DSP Lab

The DSP Lab (`#/dsp`) covers the signal processing Iris needs: spectra and windows, FIR and IIR filter design, fixed-point effects, the
**PDM microphone decimation chain**, and the **two-microphone differential beamformer**. All the maths runs in Rust
(`crates/forge-core/src/dsp/`), compiled to WebAssembly. The page draws the plots, animates them and plays audio.

| Tab | What you do there |
|---|---|
| **Spectrum** | Build a signal from tones, noise, chirps, square/saw waves (naive or band-limited) and a synthetic vowel. Pick a window, zero-pad it and read the coherent-gain-corrected spectrum. **Play** it through Web Audio. The leakage demo sweeps a tone between bins and reproduces Harris' window table numerically. |
| **Filter designer** | FIR (windowed sinc, Kaiser from specs, weighted least squares) or IIR (Butterworth by bilinear transform, RBJ cookbook biquads). Shows magnitude, unwrapped phase, exact group delay, the z-plane pole–zero plot, and the impulse and step responses. You can apply the filter to the current signal and listen before/after, compare fixed point against double precision, and download CSV, a C header or JSON. |
| **PDM mic chain** | Animated 1-bit stream, the noise-shaped spectrum with the ideal NTF, the CIC / half-band / FIR responses with the alias bands shaded, the droop compensation, and the decimated 16 kHz output with its SNDR. |
| **Beamformer** | A polar pattern that morphs as you change the pattern, delay, spacing and frequency. Also DI and WNG against frequency, the on-axis response with its EQ, a comparison table of the named patterns, and a side view of the head with a draggable sound source over a near-field level map. |

## Engine API (`dsp.*`)

| Method | Input → output |
|---|---|
| `dsp.signal` | `{fs, n, components[]}` → `{fs, x, rms, peak}` |
| `dsp.analyze` | `{spec, window, beta, nfft}` → `{signal, spectrum}` |
| `dsp.spectrum` | `{x, fs, window, beta, nfft}` → amplitude spectrum (`freq, amp, db, bin_hz, coherent_gain, enbw_bins, peaks`) |
| `dsp.psd` | `{x, fs, window, seg_len, overlap}` → Welch PSD (units²/Hz, dB/Hz, total power) |
| `dsp.fft` | `{re, im, inverse}` → `{re, im, n}` (radix-2, zero-padded) |
| `dsp.window` | `{kind, n, beta}` → samples + coherent gain, ENBW, scalloping loss, peak side lobe, main-lobe width, transform |
| `dsp.leakage` | `{n, bin, windows[]}` → per-window zero-padded DFT traces, peak reading, leakage |
| `dsp.fir_design` | `{band, method: window\|kaiser\|ls, window, beta, taps, f1, f2, fs, atten_db, transition_hz}` → `FilterReport` |
| `dsp.iir_design` | `{family: butterworth\|rbj, biquad, order, highpass, f0, q, gain_db, fs}` → `FilterReport` (with `sections`) |
| `dsp.filter_response` | `{b, a}` or `{sos}` → `FilterReport` |
| `dsp.apply` | `{b, a}` or `{sos}`, `x` → `{y}` (DF-II transposed) |
| `dsp.fixed_point` | `{b, a \| sos, coeff_bits, data_bits, x}` → Q formats, float vs fixed response, stability, output SNR |
| `dsp.pdm_chain` | `{project \| audio, chain, sim, coeff_bits}` → the full `ChainSim` (design, responses, bitstream, spectra, output, SNDR, min taps) |
| `dsp.decimation_design` | `{project \| audio, chain}` → the chain design and its alias attenuation (cheap, used by the Chip Lab) |
| `dsp.beamformer` | `{spacing_mm, pattern, tau_ratio, polar_freqs, f_min, f_max, eq_low_hz}` → polar patterns, DI/WNG vs frequency, on-axis response and EQ, pattern table |
| `dsp.beam_field` | `{spacing_mm, pattern, freq, tilt_deg, grid, probes}` → near-field level map and probe levels |

`FilterReport` = `{b, a, sections, freq, mag_db, phase_deg, group_delay, impulse, step, poles, zeros, origin_poles, stable, max_pole_radius, taps, order, beta, notes}`.

### Metrics

| Key | Iris value | Requirement |
|---|---|---|
| `dsp.mic_di_db` | **6.00 dB**: hypercardioid, d = 14 mm, 1 kHz (the ideal limit is 6.02 dB); white-noise gain −12.4 dB | R-009 ≥ 4 dB ✓ |
| `dsp.decim_stopband_db` | **67.6 dB**: CIC N=4, R=32, M=1, then a 31-tap least-squares compensation FIR ↓2. The worst case is at 28 kHz, and 12-bit coefficients also give 67.6 dB | R-010 ≥ 60 dB ✓ |

**How Iris chooses its beam pattern.** Iris uses the **hypercardioid**. R-009 asks for rejection of *diffuse* noise, and the directivity
index measures exactly that. Among first-order differential arrays the hypercardioid (α = ¼, τ = d/3c = 13.6 µs) has the highest DI.
The supercardioid gives the best front-to-back ratio, but its DI is 0.3 dB lower.

**About the PDM chain.** The chain meets R-010 with margin, and the CIC sets the limit, not the FIR. Frequencies within ±4 kHz of
k·32 kHz fall in the FIR's pass band, because its response repeats every 32 kHz. So only the CIC can reject them: its alias-band edge
attenuation is 4 × 17.1 dB = 68.5 dB. With N = 4, only **9 taps** would reach 60 dB (the page reports this live). The 31-tap default is
therefore generous. Shortening it shrinks the IRIS-DSP chip, because each tap costs a 16-bit register in the MAC's sample buffer. A
3rd-order CIC can never reach 60 dB (its floor is ~51 dB), whatever the FIR length.

## The maths

- **DFT and FFT**: X[k] = Σ x[n] w[n] e^{−j2πkn/N}. The FFT is iterative radix-2 decimation-in-time: bit-reversal permutation, then
  log₂N butterfly stages, with twiddles re-anchored every 64 steps for accuracy. Amplitudes are divided by Σw (the coherent gain), so a
  bin-centred sine reads its true amplitude. The Welch PSD divides by fs·Σw², and its integral equals the mean square (checked by test).
- **Windows**: Hann, Hamming, Blackman and Kaiser (I₀ by power series). The figures of merit are computed numerically: coherent gain,
  ENBW, scalloping loss, peak side lobe and main-lobe width.
- **FIR**: the windowed sinc places the ideal edge at −6 dB. Kaiser's formulas give β = 0.1102(A−8.7) for A > 50 dB and
  M = (A−8)/(2.285Δω). The weighted least-squares design solves the normal equations for a type-I cosine series.
- **IIR**: RBJ cookbook biquads. The Butterworth design places poles at Ω_c e^{jπ(2k+N+1)/2N}, pre-warps Ω_c = 2fs·tan(πfc/fs) and
  applies the bilinear transform per section. Poles and zeros come from exact per-section quadratics, or from Aberth–Ehrlich iteration
  for long polynomials. The filter is stable when every pole has |z| < 1.
- **Group delay**: computed exactly as τ = Re{Σ k c_k e^{−jωk} / Σ c_k e^{−jωk}} for B and A, not by differentiating the phase.
- **Fixed point**: Q(int).(frac) formats are sized to the largest coefficient. The ideal SNR is 6.02B + 1.76 dB. IIR sections are
  quantised and re-quantised between stages.
- **Sigma-delta modulation**: error-feedback form with NTF (1−z⁻¹)^L, L = 1 or 2, and ±1 output.
- **CIC**: H(z) = [(1 − z^−RM)/(1 − z⁻¹)]^N. It is implemented in integers with Hogenauer's register width B_in + N log₂(RM) and
  two's-complement wrap (a test shows wrap-around is harmless).
- **Compensation FIR**: a least-squares design with target D(f) = 1/|H_CIC(f)| (× the half-band stage, if present) in the pass band,
  weight 200 in the stop band, and a don't-care transition band.
- **Alias metric**: for every band [k·fs_out − f_voice, k·fs_out + f_voice] below the PDM Nyquist frequency, the complete-chain
  response is sampled at 41 points. The metric is the worst attenuation relative to DC.
- **Beamformer**: H(ω,θ) = w₁ + w₂ e^{−jωd cosθ/c}. For delay-and-subtract, w = (1, −e^{−jωτ}); for delay-and-sum, w = ½(e^{−jωd/c}, 1).
  DI is found by Simpson integration over the sphere; WNG = |wᴴd|²/wᴴw. The field map uses spherical waves e^{−jkr}/r, so it shows the
  near-field proximity boost at the mouth.

## Tests (`cargo test -p forge-core dsp`)

- An FFT of a pure tone lands in the right bin with the right amplitude; inverse-FFT round trip; Parseval's theorem; Welch power.
- Window coherent gains (1, 0.5, 0.54, 0.42), Hann ENBW of 1.5 bins, scalloping loss (1.42 / 3.92 dB), side lobes (−31.5, −13.3, −42.7 dB), and I₀ against tables.
- The FIR cut-off sits at −6.02 dB; high-pass and band-pass shapes; Kaiser β = 5.653 at 60 dB and the design meets 60 dB; least squares.
- Butterworth is −3.01 dB at fc for orders 1–8; RBJ gains (notch 0, peaking +6 dB, shelves); stability detection; SOS → transfer function.
- CIC DC gain (RM)^N; wrap-around; the 28 kHz hand calculation (−68.4 dB); noise shaping with both modulator orders.
- The Iris chain passes 60 dB with SNDR > 60 dB; the minimum tap count is exact; a 3rd-order CIC cannot pass.
- Small-spacing DI: cardioid 4.77, hypercardioid 6.02, supercardioid 5.72, dipole 4.77 dB (Elko; Benesty & Chen). Also the
  hypercardioid null at 109.47°, the WNG formula, the +6 dB/octave slope and the far-field map null.
- The Iris metrics fall within bounds; dispatch round trips; long-FIR roots satisfy a backward-error bound.

## How Iris uses it

- **SYS-AUD (voice capture)**: two PDM MEMS microphones (MK1, MK2) sit 14 mm apart on the right temple. The **Mic spacing** and
  **Final FIR taps** sliders write to `design.audio`, so R-009 and R-010 re-verify live.
- **SYS-ASIC**: the decimation FIR designed here is the one the Chip Lab quantises, turns into Verilog and packages for Tiny Tapeout.

## Limitations

- The ΣΔ modulator is an ideal 1st/2nd-order error-feedback loop. Real PDM microphones use 4th-order modulators with dither and
  nonlinear effects, so SNDR here is not a microphone specification.
- The alias metric covers filter attenuation only. It does not model microphone noise or the ADC's own aliasing.
- The beamformer uses free-field point sources with ideal, matched omnidirectional microphones: no head shadowing, mismatch, wind or
  reverberation. DI uses an axisymmetric array. The head drawing is illustrative only.
- The FIR designs are windowed, Kaiser or least-squares; Parks–McClellan (equiripple) is not implemented. Filtering is offline (whole
  buffers), not streaming.

## Graduating to professional tools

- **SciPy.signal**: `firwin`, `kaiserord`, `firls`, `remez`, `butter(output='sos')`, `freqz`, `group_delay`, `welch`,
  `resample_poly`. You can cross-check every number above.
- **GNU Radio**: real-time streaming flowgraphs with SDR hardware, polyphase decimators and filter-design tools.
- **Faust**: a functional DSP language that compiles to C++, WebAssembly and plugins, suited to audio-rate prototypes of the
  beamformer and EQ.

## References

- A. V. Oppenheim & R. W. Schafer, *Discrete-Time Signal Processing*, 3rd ed., Pearson 2010.
- J. W. Cooley & J. W. Tukey, "An algorithm for the machine calculation of complex Fourier series", *Math. Comp.* 19, 1965.
- F. J. Harris, "On the use of windows for harmonic analysis with the discrete Fourier transform", *Proc. IEEE* 66(1), 1978.
- J. F. Kaiser, "Nonrecursive digital filter design using the I0-sinh window function", Proc. IEEE ISCAS 1974.
- R. Bristow-Johnson, "Cookbook formulae for audio EQ biquad filter coefficients", W3C Note — https://www.w3.org/TR/audio-eq-cookbook/
- E. B. Hogenauer, "An economical class of digital filters for decimation and interpolation", *IEEE Trans. ASSP* 29(2), 1981.
- R. Schreier & G. C. Temes, *Understanding Delta-Sigma Data Converters*, Wiley 2005.
- G. W. Elko, "Superdirectional microphone arrays", in *Acoustic Signal Processing for Telecommunication*, Kluwer 2000.
- J. Benesty & J. Chen, *Study and Design of Differential Microphone Arrays*, Springer 2013.
- P. D. Welch, "The use of fast Fourier transform for the estimation of power spectra", *IEEE Trans. Audio Electroacoust.* 15, 1967.
- O. Aberth, "Iteration methods for finding all zeros of a polynomial simultaneously", *Math. Comp.* 27, 1973.
- G. E. Peterson & H. L. Barney, "Control methods used in a study of the vowels", *JASA* 24, 1952 (formant values).
