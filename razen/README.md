# Razen Compiler Workspace

<p align="center">
  <!-- Replace with your actual logo path -->
  <img src="/assets/logos/razen_logo_light.png" alt="Razen Logo" width="160">
</p>

<h1 align="center">Razen Programming Language</h1>

<p align="center">
  <b>Modern. Fast. Reliable.</b><br>
  A statically typed systems language bridging the gap between rapid development and high-performance production.
</p>

<p align="center">
  <!-- Example Badges - Remove or update links as needed -->
  <img src="https://img.shields.io/badge/license-Apache%202.0-blue.svg" alt="License">
  <img src="https://img.shields.io/badge/platform-Linux%20%7C%20Windows%20%7C%20macOS-lightgrey" alt="Platform">
  <img src="https://img.shields.io/badge/status-Active%20Development-success" alt="Status">
  <a href="https://aratt.ai/@razen"><img src="https://img.shields.io/badge/Arattai-Join%20Channel-25D366?style=flat&logo=arattai&logoColor=white" alt="Join Arattai Channel"></a>
</p>

<p align="center">
  <b>Razen</b> (formerly <b>Razen</b>) is the official language core under active development.
</p>

<br>

Razen is a modern, statically typed programming language designed to give professional developers the control of systems programming with the ergonomics needed for rapid iteration and high‑performance production deployment.

## Architecture and Compilation

Razen functions as a high-level frontend that transpiles to **C11-23**, offering a unique dual-pipeline approach. This architecture allows Razen to remain lightweight while leveraging decades of existing compiler optimizations.

### Build Modes
Razen distinguishes between development speed and execution speed by automatically switching backend compilers:

<table>
  <tr>
    <th width="50%">Rapid Development</th>
    <th width="50%">Production Builds</th>
  </tr>
  <tr>
    <td><b>Command:</b> <code>razen run</code></td>
    <td><b>Command:</b> <code>razen build</code></td>
  </tr>
  <tr>
    <td>Uses <b>TCC (Tiny C Compiler)</b> for near-instant compilation. Mimics the feedback loop of an interpreted language while executing native code.</td>
    <td>Targets <b>GCC / Clang</b> for maximum optimization. Produces highly efficient, standalone binaries suitable for deployment.</td>
  </tr>
</table>

## Community

Stay updated with the latest development news, feature polls, and announcements.

- **Official Channel:** [Join us on Arattai](https://aratt.ai/@razen)

## Roadmap

We have a structured plan to bring Razen from Alpha to a production-ready state. Please refer to our official roadmap documentation for detailed milestones and active development phases.

<br>

---

## Copyright and Licensing

<p align="left">
  <strong>Copyright © 2025 Prathmesh Barot & Razen Core Team.</strong><br>
  This project is licensed under the <strong>Apache License, Version 2.0</strong>.
</p>

> **Trademark Notice:**
> The "Razen" name, the Razen logo, and all related branding assets are exclusively owned by the Razen Core Team and Prathmesh Barot. They are **not** covered by the Apache License 2.0. You may not use these trademarks to endorse or promote products derived from this software without specific prior written permission.

**Custom Usage Notice:**
Unless required by applicable law or agreed to in writing, software distributed under this License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the [LICENSE](LICENSE) file for the specific language governing permissions and limitations under the License.