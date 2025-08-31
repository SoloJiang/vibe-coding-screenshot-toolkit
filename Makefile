# Git hooks 和代码质量检查的便利命令

# 安装 git hooks
.PHONY: install-hooks
install-hooks:
	@echo "🔧 Installing git hooks..."
	@chmod +x .git/hooks/pre-commit
	@echo "✅ Git hooks installed successfully!"

# 运行代码格式化
.PHONY: fmt
fmt:
	@echo "📝 Formatting code with cargo fmt..."
	@cargo fmt
	@echo "✅ Code formatting completed!"

# 检查代码格式
.PHONY: fmt-check
fmt-check:
	@echo "📝 Checking code formatting..."
	@cargo fmt -- --check
	@echo "✅ Code formatting check passed!"

# 运行 clippy 检查
.PHONY: clippy
clippy:
	@echo "🔧 Running cargo clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings
	@echo "✅ Clippy check passed!"

# 运行所有质量检查（格式化和 clippy）
.PHONY: check
check: fmt-check clippy
	@echo "🎉 All code quality checks passed!"

# 自动修复代码（格式化）
.PHONY: fix
fix: fmt
	@echo "🔧 Auto-fixing code issues..."
	@cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged
	@echo "✅ Auto-fix completed!"

# 运行测试
.PHONY: test
test:
	@echo "🧪 Running tests..."
	@cargo test
	@echo "✅ Tests completed!"

# 完整的 CI 检查（格式化、clippy、测试）
.PHONY: ci
ci: check test
	@echo "🎉 All CI checks passed!"

# 清理构建产物
.PHONY: clean
clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@echo "✅ Clean completed!"

# 显示帮助信息
.PHONY: help
help:
	@echo "Available commands:"
	@echo "  install-hooks - Install git hooks"
	@echo "  fmt          - Format code with cargo fmt"
	@echo "  fmt-check    - Check code formatting"
	@echo "  clippy       - Run cargo clippy checks"
	@echo "  check        - Run all quality checks (fmt-check + clippy)"
	@echo "  fix          - Auto-fix code issues"
	@echo "  test         - Run tests"
	@echo "  ci           - Run complete CI checks"
	@echo "  clean        - Clean build artifacts"
	@echo "  help         - Show this help message"
