Name: bear
Version: 0.3
Release: 1%{?dist}

Summary: BuildEAR
License: MIT
Group: Development/Tools
URL: http://github.com/rizsotto/Bear

Source: %{name}-%{version}.tar.gz

Prefix: %{_prefix}
BuildRoot: %{_tmppath}/%{name}-%{version}-root

BuildRequires:  gcc, cmake, make

%description
Bear is a tool to generate compilation database for clang tooling.

%clean
%{__rm} -rf %{buildroot}

%prep
%setup

%build
%{__cmake} . -DCMAKE_INSTALL_PREFIX=%{_prefix}
%{__make} check

%install
%{__make} install DESTDIR=%{buildroot}

%files
%{_bindir}/bear
%{_libdir}/libear.so

%post

%preun

%changelog
* Wed Jan 2 2013 Laszlo Nagy <rizsotto@gmail.com> - 0.3-1
- Initial build
