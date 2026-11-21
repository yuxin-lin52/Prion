
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum, IntType, StructType};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValueEnum, FunctionValue, GlobalValue, IntValue, PointerValue,
};
use inkwell::{AddressSpace, IntPredicate};

pub fn 发射<'ctx, 'a>(
    ctx: &'ctx Context,
    m: &'a Module<'ctx>,
    windows: bool,
    release: bool,
    file: &str,
) -> Result<(), String> {
    let 器 = 发射器 {
        ctx,
        m,
        b: ctx.create_builder(),
        windows,
        release,
        源文件: file.to_string(),
    };
    器.打印();
    器.错误();
    器.参数();
    器.文件();
    器.字符串();
    器.容器(release)?;
    器.进程()?;
    器.系统();
    器.并发()?;
    Ok(())
}

struct 发射器<'ctx, 'a> {
    ctx: &'ctx Context,
    m: &'a Module<'ctx>,
    b: Builder<'ctx>,
    windows: bool,
    release: bool,
    源文件: String,
}

impl<'ctx, 'a> 发射器<'ctx, 'a> {
    fn i8t(&self) -> IntType<'ctx> {
        self.ctx.i8_type()
    }

    fn i32t(&self) -> IntType<'ctx> {
        self.ctx.i32_type()
    }

    fn i64t(&self) -> IntType<'ctx> {
        self.ctx.i64_type()
    }

    fn pt(&self) -> inkwell::types::PointerType<'ctx> {
        self.ctx.ptr_type(AddressSpace::default())
    }

    fn p(&self) -> BasicTypeEnum<'ctx> {
        self.pt().into()
    }

    fn 串型(&self) -> StructType<'ctx> {
        if let Some(t) = self.ctx.get_struct_type("PrionStr") {
            return t;
        }
        let t = self.ctx.opaque_struct_type("PrionStr");
        t.set_body(&[self.pt().into(), self.i64t().into()], false);
        t
    }

    fn 列型(&self) -> StructType<'ctx> {
        if let Some(t) = self.ctx.get_struct_type("PrionList") {
            return t;
        }
        let t = self.ctx.opaque_struct_type("PrionList");
        t.set_body(
            &[
                self.i64t().into(),
                self.i64t().into(),
                self.i64t().into(),
                self.i64t().into(),
                self.pt().into(),
            ],
            false,
        );
        t
    }

    fn 函数(
        &self,
        name: &str,
        ret: Option<BasicTypeEnum<'ctx>>,
        ps: &[BasicTypeEnum<'ctx>],
        va: bool,
    ) -> FunctionValue<'ctx> {
        if let Some(f) = self.m.get_function(name) {
            return f;
        }
        let mps: Vec<BasicMetadataTypeEnum<'ctx>> = ps.iter().map(|t| (*t).into()).collect();
        let fty = match ret {
            None => self.ctx.void_type().fn_type(&mps, va),
            Some(BasicTypeEnum::IntType(t)) => t.fn_type(&mps, va),
            Some(BasicTypeEnum::FloatType(t)) => t.fn_type(&mps, va),
            Some(BasicTypeEnum::PointerType(t)) => t.fn_type(&mps, va),
            Some(BasicTypeEnum::StructType(t)) => t.fn_type(&mps, va),
            Some(_) => self.ctx.void_type().fn_type(&mps, va),
        };
        let f = self.m.add_function(name, fty, None);
        let kind = inkwell::attributes::Attribute::get_named_enum_kind_id("noinline");
        f.add_attribute(
            inkwell::attributes::AttributeLoc::Function,
            self.ctx.create_enum_attribute(kind, 0),
        );
        f
    }

    fn 入口(&self, f: FunctionValue<'ctx>) -> bool {
        if f.get_first_basic_block().is_some() {
            return false;
        }
        let bb = self.ctx.append_basic_block(f, "entry");
        self.b.position_at_end(bb);
        true
    }

    fn 调(
        &self,
        f: FunctionValue<'ctx>,
        args: &[BasicValueEnum<'ctx>],
        name: &str,
    ) -> Option<BasicValueEnum<'ctx>> {
        let av: Vec<BasicMetadataValueEnum<'ctx>> = args.iter().map(|a| (*a).into()).collect();
        self.b
            .build_call(f, &av, name)
            .unwrap()
            .try_as_basic_value()
            .basic()
    }

    fn 串(&self, s: &str) -> PointerValue<'ctx> {
        self.b.build_global_string_ptr(s, "rt.str").unwrap().as_pointer_value()
    }

    fn 外i64(&self, name: &str) -> GlobalValue<'ctx> {
        if let Some(g) = self.m.get_global(name) {
            return g;
        }
        let g = self.m.add_global(self.i64t(), None, name);
        g.set_linkage(Linkage::External);
        g
    }

    fn 外ptr(&self, name: &str) -> GlobalValue<'ctx> {
        if let Some(g) = self.m.get_global(name) {
            return g;
        }
        let g = self.m.add_global(self.pt(), None, name);
        g.set_linkage(Linkage::External);
        g
    }

    fn 内部i32(&self, name: &str) -> GlobalValue<'ctx> {
        if let Some(g) = self.m.get_global(name) {
            return g;
        }
        let g = self.m.add_global(self.i32t(), None, name);
        g.set_linkage(Linkage::Internal);
        g.set_initializer(&self.i32t().const_zero());
        g
    }

    fn 内部ptr(&self, name: &str) -> GlobalValue<'ctx> {
        if let Some(g) = self.m.get_global(name) {
            return g;
        }
        let g = self.m.add_global(self.pt(), None, name);
        g.set_linkage(Linkage::Internal);
        g.set_initializer(&self.pt().const_null());
        g
    }

    fn 错误流(&self) -> PointerValue<'ctx> {
        if self.windows {
            let f = self.函数("__acrt_iob_func", Some(self.p()), &[self.i32t().into()], false);
            self.调(f, &[self.i32t().const_int(2, false).into()], "err")
                .unwrap()
                .into_pointer_value()
        } else {
            let g = self.外ptr("stderr");
            self.b.build_load(self.p(), g.as_pointer_value(), "err").unwrap().into_pointer_value()
        }
    }

    fn fprintf(&self) -> FunctionValue<'ctx> {
        self.函数("fprintf", Some(self.i32t().into()), &[self.p(), self.p()], true)
    }

    fn fputc(&self) -> FunctionValue<'ctx> {
        self.函数("fputc", Some(self.i32t().into()), &[self.i32t().into(), self.p()], false)
    }

    fn malloc(&self) -> FunctionValue<'ctx> {
        self.函数("malloc", Some(self.p()), &[self.i64t().into()], false)
    }

    fn free(&self) -> FunctionValue<'ctx> {
        self.函数("free", None, &[self.p()], false)
    }

    fn strlen(&self) -> FunctionValue<'ctx> {
        self.函数("strlen", Some(self.i64t().into()), &[self.p()], false)
    }

    fn exit(&self) -> FunctionValue<'ctx> {
        self.函数("exit", None, &[self.i32t().into()], false)
    }

    fn die_nc_fn(&self) -> FunctionValue<'ctx> {
        self.函数(
            "prion_die_nc",
            None,
            &[self.i32t().into(), self.p(), self.p()],
            false,
        )
    }

    fn 失败通知(
        &self,
        code: IntValue<'ctx>,
        line: IntValue<'ctx>,
        msg: PointerValue<'ctx>,
        fix: PointerValue<'ctx>,
    ) {
        let f = self.函数(
            "prion_task_fail_notify",
            None,
            &[self.i32t().into(), self.i64t().into(), self.p(), self.p()],
            false,
        );
        self.调(f, &[code.into(), line.into(), msg.into(), fix.into()], "");
    }

    fn 报错(&self, fmt: &str, args: &[BasicValueEnum<'ctx>]) {
        let ep = self.错误流();
        let fs = self.串(fmt);
        let mut av: Vec<BasicValueEnum<'ctx>> = vec![ep.into(), fs.into()];
        av.extend_from_slice(args);
        self.调(self.fprintf(), &av, "");
    }

    fn 打印(&self) {
        self.bool_str();
        self.at();
        self.print_raw();
    }

    fn bool_str(&self) {
        let f = self.函数("prion_bool_str", None, &[self.p(), self.i64t().into()], false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let v = f.get_nth_param(1).unwrap().into_int_value();
        let c = self.b.build_int_compare(IntPredicate::NE, v, self.i64t().const_zero(), "b").unwrap();
        let re = self.串("re");
        let fa = self.串("fa");
        let pv = self.b.build_select(c, re, fa, "bp").unwrap().into_pointer_value();
        let st = self.串型();
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, pv, 0, "s0").unwrap().into_struct_value();
        sv = self
            .b
            .build_insert_value(sv, self.i64t().const_int(2, false), 1, "s1")
            .unwrap()
            .into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn at(&self) {
        let f = self.函数("prion_at", None, &[], false);
        if !self.入口(f) {
            return;
        }
        let g = self.内部i64("prion_line");
        let line = self.b.build_load(self.i64t(), g.as_pointer_value(), "ln").unwrap();
        self.报错("  (line %lld)\n", &[line]);
        self.b.build_return(None).unwrap();
    }

    fn 错误(&self) {
        self.die_at();
        self.die();
        self.die_nc();
    }

    fn die_at(&self) {
        let ps = [
            self.p(),
            self.i64t().into(),
            self.i64t().into(),
            self.p(),
            self.i32t().into(),
            self.p(),
            self.p(),
        ];
        let f = self.函数("prion_die_at", None, &ps, false);
        if !self.入口(f) {
            return;
        }
        let 首块 = f.get_first_basic_block().unwrap();
        let file = f.get_nth_param(0).unwrap().into_pointer_value();
        let line = f.get_nth_param(1).unwrap().into_int_value();
        let col = f.get_nth_param(2).unwrap().into_int_value();
        let src = f.get_nth_param(3).unwrap().into_pointer_value();
        let code = f.get_nth_param(4).unwrap().into_int_value();
        let msg = f.get_nth_param(5).unwrap().into_pointer_value();
        let fix = f.get_nth_param(6).unwrap().into_pointer_value();
        self.失败通知(code, line, msg, fix);

        let b_null = self.ctx.append_basic_block(f, "fnull");
        let b_chk = self.ctx.append_basic_block(f, "fchk");
        let b_has = self.ctx.append_basic_block(f, "fhas");
        let isnull = self.b.build_is_null(file, "fn").unwrap();
        self.b.build_conditional_branch(isnull, b_null, b_chk).unwrap();
        self.b.position_at_end(b_null);
        self.b.build_unconditional_branch(b_has).unwrap();
        self.b.position_at_end(b_chk);
        let c0 = self.b.build_load(self.i8t(), file, "c0").unwrap().into_int_value();
        let nz = self.b.build_int_compare(IntPredicate::NE, c0, self.i8t().const_zero(), "nz").unwrap();
        self.b.build_unconditional_branch(b_has).unwrap();
        self.b.position_at_end(b_has);
        let has = self.b.build_phi(self.ctx.bool_type(), "has").unwrap();
        has.add_incoming(&[
            (&self.ctx.bool_type().const_zero(), b_null),
            (&nz, b_chk),
        ]);
        let hasi = has.as_basic_value().into_int_value();
        let 文件 = self.串(&self.源文件);
        let srcf = self.b.build_select(hasi, file, 文件, "srcf").unwrap();
        let 有行 = self.b.build_int_compare(IntPredicate::SGT, line, self.i64t().const_zero(), "hl").unwrap();
        let cond = self.b.build_or(hasi, 有行, "cond").unwrap();

        let b_show = self.ctx.append_basic_block(f, "fshow");
        let b_plain = self.ctx.append_basic_block(f, "fplain");
        let b_after1 = self.ctx.append_basic_block(f, "fa1");
        self.b.build_conditional_branch(cond, b_show, b_plain).unwrap();

        self.b.position_at_end(b_show);
        self.报错("[R%02d] at %s", &[code.into(), srcf]);
        self.b.build_unconditional_branch(b_after1).unwrap();
        self.b.position_at_end(b_plain);
        self.报错("[R%02d]", &[code.into()]);
        self.b.build_unconditional_branch(b_after1).unwrap();

        self.b.position_at_end(b_after1);
        let b_loc = self.ctx.append_basic_block(f, "floc");
        let b_noline = self.ctx.append_basic_block(f, "fnl");
        self.b.build_conditional_branch(有行, b_loc, b_noline).unwrap();

        self.b.position_at_end(b_loc);
        let b_col = self.ctx.append_basic_block(f, "fcol");
        let b_nocol = self.ctx.append_basic_block(f, "fnc");
        let 有列 = self.b.build_int_compare(IntPredicate::SGT, col, self.i64t().const_zero(), "hc").unwrap();
        self.b.build_conditional_branch(有列, b_col, b_nocol).unwrap();
        self.b.position_at_end(b_col);
        self.报错(" error at line %lld, column %lld", &[line.into(), col.into()]);
        self.b.build_unconditional_branch(b_nocol).unwrap();
        self.b.position_at_end(b_nocol);
        self.报错(" error at line %lld", &[line.into()]);
        let b_src = self.ctx.append_basic_block(f, "fsrc");
        self.b.build_unconditional_branch(b_src).unwrap();

        self.b.position_at_end(b_src);
        let b_srcshow = self.ctx.append_basic_block(f, "fss");
        let b_srcload = self.ctx.append_basic_block(f, "fsl");
        let b_end = self.ctx.append_basic_block(f, "fend");
        let snull = self.b.build_is_null(src, "sn").unwrap();
        self.b.build_conditional_branch(snull, b_end, b_srcload).unwrap();
        self.b.position_at_end(b_srcload);
        let sc0 = self.b.build_load(self.i8t(), src, "sc").unwrap().into_int_value();
        let snz = self.b.build_int_compare(IntPredicate::NE, sc0, self.i8t().const_zero(), "snz").unwrap();
        self.b.build_conditional_branch(snz, b_srcshow, b_end).unwrap();
        self.b.position_at_end(b_srcshow);
        self.报错(", source: %s", &[src.into()]);
        self.b.build_unconditional_branch(b_end).unwrap();

        self.b.position_at_end(b_noline);
        self.报错("error occurred", &[]);
        self.b.build_unconditional_branch(b_end).unwrap();

        self.b.position_at_end(b_end);
        self.报错(", error: %s. Fix: %s.\n", &[msg.into(), fix.into()]);
        self.调(self.exit(), &[self.i32t().const_int(1, false).into()], "");
        self.b.build_return(None).unwrap();
        let _ = 首块;
    }

    fn die(&self) {
        let ps = [self.i32t().into(), self.i64t().into(), self.p(), self.p()];
        let f = self.函数("prion_die", None, &ps, false);
        if !self.入口(f) {
            return;
        }
        let code = f.get_nth_param(0).unwrap().into_int_value();
        let line = f.get_nth_param(1).unwrap().into_int_value();
        let msg = f.get_nth_param(2).unwrap().into_pointer_value();
        let fix = f.get_nth_param(3).unwrap().into_pointer_value();
        self.失败通知(code, line, msg, fix);

        let g = self.内部ptr("prion_cur_file");
        let cur = self.b.build_load(self.p(), g.as_pointer_value(), "cf").unwrap().into_pointer_value();
        let 文件 = self.串(&self.源文件);
        let srcf = self
            .b
            .build_select(self.b.build_is_null(cur, "cn").unwrap(), 文件, cur, "srcf")
            .unwrap();
        self.报错("[R%02d] at %s", &[code.into(), srcf]);

        let b_loc = self.ctx.append_basic_block(f, "dl");
        let b_noline = self.ctx.append_basic_block(f, "dnl");
        let 有行 = self.b.build_int_compare(IntPredicate::SGT, line, self.i64t().const_zero(), "hl").unwrap();
        self.b.build_conditional_branch(有行, b_loc, b_noline).unwrap();

        self.b.position_at_end(b_loc);
        let gc = self.内部i64("prion_col");
        let col = self.b.build_load(self.i64t(), gc.as_pointer_value(), "col").unwrap().into_int_value();
        let b_col = self.ctx.append_basic_block(f, "dc");
        let b_nocol = self.ctx.append_basic_block(f, "dnc");
        let 有列 = self.b.build_int_compare(IntPredicate::SGT, col, self.i64t().const_zero(), "hc").unwrap();
        self.b.build_conditional_branch(有列, b_col, b_nocol).unwrap();
        self.b.position_at_end(b_col);
        self.报错(" error at line %lld, column %lld", &[line.into(), col.into()]);
        self.b.build_unconditional_branch(b_nocol).unwrap();
        self.b.position_at_end(b_nocol);
        self.报错(" error at line %lld", &[line.into()]);
        let b_src = self.ctx.append_basic_block(f, "ds");
        self.b.build_unconditional_branch(b_src).unwrap();

        self.b.position_at_end(b_src);
        let b_srcshow = self.ctx.append_basic_block(f, "dss");
        let b_end = self.ctx.append_basic_block(f, "de");
        let gs = self.内部ptr("prion_src");
        let src = self.b.build_load(self.p(), gs.as_pointer_value(), "sv").unwrap().into_pointer_value();
        self.b.build_conditional_branch(
            self.b.build_is_null(src, "sn").unwrap(),
            b_end,
            b_srcshow,
        ).unwrap();
        self.b.position_at_end(b_srcshow);
        self.报错(", source: %s", &[src.into()]);
        self.b.build_unconditional_branch(b_end).unwrap();

        self.b.position_at_end(b_noline);
        self.报错("error occurred", &[]);
        self.b.build_unconditional_branch(b_end).unwrap();

        self.b.position_at_end(b_end);
        self.报错(", error: %s. Fix: %s.\n", &[msg.into(), fix.into()]);
        self.调(self.exit(), &[self.i32t().const_int(1, false).into()], "");
        self.b.build_return(None).unwrap();
    }

    fn die_nc(&self) {
        let f = self.die_nc_fn();
        if !self.入口(f) {
            return;
        }
        let code = f.get_nth_param(0).unwrap().into_int_value();
        let msg = f.get_nth_param(1).unwrap().into_pointer_value();
        let fix = f.get_nth_param(2).unwrap().into_pointer_value();
        let g = self.内部i64("prion_line");
        let line = self.b.build_load(self.i64t(), g.as_pointer_value(), "ln").unwrap().into_int_value();
        let d = self.函数("prion_die", None, &[self.i32t().into(), self.i64t().into(), self.p(), self.p()], false);
        self.调(d, &[code.into(), line.into(), msg.into(), fix.into()], "");
        self.b.build_return(None).unwrap();
    }

    fn 参数(&self) {
        self.内部i32("prion_argc");
        self.内部ptr("prion_argv");
        self.args_set();
        self.args();
    }

    fn args_set(&self) {
        let f = self.函数("prion_args_set", None, &[self.i32t().into(), self.p()], false);
        if !self.入口(f) {
            return;
        }
        let argc = f.get_nth_param(0).unwrap().into_int_value();
        let argv = f.get_nth_param(1).unwrap().into_pointer_value();
        let g = self.m.get_global("prion_argc").unwrap();
        self.b.build_store(g.as_pointer_value(), argc).unwrap();
        let gp = self.m.get_global("prion_argv").unwrap();
        self.b.build_store(gp.as_pointer_value(), argv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn args(&self) {
        let f = self.函数("prion_args", Some(self.p()), &[], false);
        if !self.入口(f) {
            return;
        }
        let 首块 = f.get_first_basic_block().unwrap();
        let st = self.串型();
        let sa = self.b.build_alloca(st, "ss").unwrap();
        let newf = self.函数(
            "prion_list_new",
            Some(self.p()),
            &[self.i64t().into(), self.i64t().into()],
            false,
        );
        let l = self
            .调(newf, &[self.i64t().const_int(16, false).into(), self.i64t().const_zero().into()], "l")
            .unwrap()
            .into_pointer_value();
        let b_loop = self.ctx.append_basic_block(f, "loop");
        let b_body = self.ctx.append_basic_block(f, "body");
        let b_done = self.ctx.append_basic_block(f, "done");
        self.b.build_unconditional_branch(b_loop).unwrap();

        self.b.position_at_end(b_loop);
        let i = self.b.build_phi(self.i64t(), "i").unwrap();
        i.add_incoming(&[(&self.i64t().const_zero(), 首块)]);
        let ga = self.m.get_global("prion_argc").unwrap();
        let argc = self.b.build_load(self.i32t(), ga.as_pointer_value(), "ac").unwrap().into_int_value();
        let argc64 = self.b.build_int_s_extend(argc, self.i64t(), "ac64").unwrap();
        let c = self
            .b
            .build_int_compare(IntPredicate::SLT, i.as_basic_value().into_int_value(), argc64, "lt")
            .unwrap();
        self.b.build_conditional_branch(c, b_body, b_done).unwrap();

        self.b.position_at_end(b_body);
        let gp = self.m.get_global("prion_argv").unwrap();
        let av = self.b.build_load(self.p(), gp.as_pointer_value(), "av").unwrap().into_pointer_value();
        let slotp = unsafe {
            self.b.build_gep(self.pt(), av, &[i.as_basic_value().into_int_value()], "ae")
        }
        .unwrap();
        let ap = self.b.build_load(self.p(), slotp, "ap").unwrap().into_pointer_value();
        let n = self.调(self.strlen(), &[ap.into()], "n").unwrap().into_int_value();
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, ap, 0, "p0").unwrap().into_struct_value();
        sv = self.b.build_insert_value(sv, n, 1, "p1").unwrap().into_struct_value();
        self.b.build_store(sa, sv).unwrap();
        let push = self.函数(
            "prion_list_push_s",
            Some(self.i64t().into()),
            &[self.p(), self.p()],
            false,
        );
        self.调(push, &[l.into(), sa.into()], "");
        let next = self
            .b
            .build_int_add(i.as_basic_value().into_int_value(), self.i64t().const_int(1, false), "in")
            .unwrap();
        i.add_incoming(&[(&next, b_body)]);
        self.b.build_unconditional_branch(b_loop).unwrap();

        self.b.position_at_end(b_done);
        self.b.build_return(Some(&l)).unwrap();
    }

    fn 文件(&self) {
        self.path_local();
        self.fopen_wrap("prion_fopen_rb", "rb");
        self.fopen_wrap("prion_fopen_wb", "wb");
        self.read_file();
        self.save_file();
    }

    fn path_local(&self) {
        let f = self.函数("prion_path_local", Some(self.p()), &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let path = f.get_nth_param(0).unwrap().into_pointer_value();
        if !self.windows {
            self.b.build_return(Some(&path)).unwrap();
            return;
        }
        let mb = self.函数(
            "MultiByteToWideChar",
            Some(self.i32t().into()),
            &[
                self.i32t().into(),
                self.i32t().into(),
                self.p(),
                self.i32t().into(),
                self.p(),
                self.i32t().into(),
            ],
            false,
        );
        let wc = self.函数(
            "WideCharToMultiByte",
            Some(self.i32t().into()),
            &[
                self.i32t().into(),
                self.i32t().into(),
                self.p(),
                self.i32t().into(),
                self.p(),
                self.i32t().into(),
                self.p(),
                self.p(),
            ],
            false,
        );
        let null = self.pt().const_null();
        let n = self
            .调(
                mb,
                &[
                    self.i32t().const_int(65001, false).into(),
                    self.i32t().const_zero().into(),
                    path.into(),
                    self.i32t().const_int(u64::MAX, true).into(),
                    null.into(),
                    self.i32t().const_zero().into(),
                ],
                "n",
            )
            .unwrap()
            .into_int_value();
        let b_ok = self.ctx.append_basic_block(f, "wok");
        let b_w2 = self.ctx.append_basic_block(f, "ww2");
        let b_w3 = self.ctx.append_basic_block(f, "ww3");
        let b_m = self.ctx.append_basic_block(f, "wm");
        let b_b = self.ctx.append_basic_block(f, "wb");
        let b_b2 = self.ctx.append_basic_block(f, "wb2");
        let b_fin = self.ctx.append_basic_block(f, "wfin");
        let b_ret = self.ctx.append_basic_block(f, "wret");

        let ok = self.b.build_int_compare(IntPredicate::SGT, n, self.i32t().const_zero(), "ok").unwrap();
        self.b.build_conditional_branch(ok, b_ok, b_ret).unwrap();

        self.b.position_at_end(b_ok);
        let n64 = self.b.build_int_s_extend(n, self.i64t(), "n64").unwrap();
        let bytes = self.b.build_int_mul(n64, self.i64t().const_int(2, false), "nb").unwrap();
        let wp = self.调(self.malloc(), &[bytes.into()], "w").unwrap().into_pointer_value();
        let b_w = self.ctx.append_basic_block(f, "ww");
        self.b.build_conditional_branch(
            self.b.build_is_null(wp, "wn").unwrap(),
            b_ret,
            b_w,
        ).unwrap();

        self.b.position_at_end(b_w);
        self.调(
            mb,
            &[
                self.i32t().const_int(65001, false).into(),
                self.i32t().const_zero().into(),
                path.into(),
                self.i32t().const_int(u64::MAX, true).into(),
                wp.into(),
                n.into(),
            ],
            "",
        );
        let m = self
            .调(
                wc,
                &[
                    self.i32t().const_zero().into(),
                    self.i32t().const_zero().into(),
                    wp.into(),
                    self.i32t().const_int(u64::MAX, true).into(),
                    null.into(),
                    self.i32t().const_zero().into(),
                    null.into(),
                    null.into(),
                ],
                "m",
            )
            .unwrap()
            .into_int_value();
        let mok = self.b.build_int_compare(IntPredicate::SGT, m, self.i32t().const_zero(), "mok").unwrap();
        self.b.build_conditional_branch(mok, b_m, b_w2).unwrap();

        self.b.position_at_end(b_w2);
        self.调(self.free(), &[wp.into()], "");
        self.b.build_unconditional_branch(b_ret).unwrap();

        self.b.position_at_end(b_m);
        let m64 = self.b.build_int_s_extend(m, self.i64t(), "m64").unwrap();
        let bp = self.调(self.malloc(), &[m64.into()], "b").unwrap().into_pointer_value();
        self.b.build_conditional_branch(
            self.b.build_is_null(bp, "bn").unwrap(),
            b_w3,
            b_b,
        ).unwrap();

        self.b.position_at_end(b_w3);
        self.调(self.free(), &[wp.into()], "");
        self.b.build_unconditional_branch(b_ret).unwrap();

        self.b.position_at_end(b_b);
        self.调(
            wc,
            &[
                self.i32t().const_zero().into(),
                self.i32t().const_zero().into(),
                wp.into(),
                self.i32t().const_int(u64::MAX, true).into(),
                bp.into(),
                m.into(),
                null.into(),
                null.into(),
            ],
            "",
        );
        self.b.build_unconditional_branch(b_b2).unwrap();

        self.b.position_at_end(b_b2);
        self.调(self.free(), &[wp.into()], "");
        self.b.build_unconditional_branch(b_fin).unwrap();

        self.b.position_at_end(b_fin);
        self.b.build_return(Some(&bp)).unwrap();

        self.b.position_at_end(b_ret);
        self.b.build_return(Some(&path)).unwrap();
    }

    fn fopen_wrap(&self, name: &str, mode: &str) {
        let f = self.函数(name, Some(self.p()), &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let path = f.get_nth_param(0).unwrap().into_pointer_value();
        let pl = self.函数("prion_path_local", Some(self.p()), &[self.p()], false);
        let lp = self.调(pl, &[path.into()], "lp").unwrap().into_pointer_value();
        let fo = self.函数("fopen", Some(self.p()), &[self.p(), self.p()], false);
        let md = self.串(mode);
        let fp = self.调(fo, &[lp.into(), md.into()], "fp").unwrap().into_pointer_value();
        if self.windows {
            let b_free = self.ctx.append_basic_block(f, "lf");
            let b_done = self.ctx.append_basic_block(f, "ld");
            let same = self
                .b
                .build_int_compare(
                    IntPredicate::EQ,
                    self.b.build_ptr_to_int(lp, self.i64t(), "lpi").unwrap(),
                    self.b.build_ptr_to_int(path, self.i64t(), "ppi").unwrap(),
                    "same",
                )
                .unwrap();
            self.b.build_conditional_branch(same, b_done, b_free).unwrap();
            self.b.position_at_end(b_free);
            self.调(self.free(), &[lp.into()], "");
            self.b.build_unconditional_branch(b_done).unwrap();
            self.b.position_at_end(b_done);
        }
        self.b.build_return(Some(&fp)).unwrap();
    }

    fn read_file(&self) {
        let f = self.函数("prion_read_file", None, &[self.p(), self.p()], false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let inp = f.get_nth_param(1).unwrap().into_pointer_value();
        let st = self.串型();
        let b_in = self.ctx.append_basic_block(f, "rin");
        let b_null = self.ctx.append_basic_block(f, "rnp");
        let b_go = self.ctx.append_basic_block(f, "rgo");
        self.b.build_conditional_branch(
            self.b.build_is_null(inp, "inn").unwrap(),
            b_null,
            b_in,
        ).unwrap();

        self.b.position_at_end(b_in);
        let pp = self.b.build_struct_gep(st, inp, 0, "pp").unwrap();
        let p = self.b.build_load(self.p(), pp, "p").unwrap().into_pointer_value();
        let e1 = self.串("");
        let path0 = self
            .b
            .build_select(self.b.build_is_null(p, "pn").unwrap(), e1, p, "psel")
            .unwrap()
            .into_pointer_value();
        self.b.build_unconditional_branch(b_go).unwrap();

        self.b.position_at_end(b_null);
        let e2 = self.串("");
        self.b.build_unconditional_branch(b_go).unwrap();

        self.b.position_at_end(b_go);
        let path = self.b.build_phi(self.p(), "rp").unwrap();
        path.add_incoming(&[(&path0, b_in), (&e2, b_null)]);
        let path = path.as_basic_value().into_pointer_value();

        let rb = self.函数("prion_fopen_rb", Some(self.p()), &[self.p()], false);
        let fp = self.调(rb, &[path.into()], "fp").unwrap().into_pointer_value();
        let b_open = self.ctx.append_basic_block(f, "rok");
        let b_fail = self.ctx.append_basic_block(f, "rf");
        self.b.build_conditional_branch(
            self.b.build_is_null(fp, "fpn").unwrap(),
            b_fail,
            b_open,
        ).unwrap();

        self.b.position_at_end(b_fail);
        self.报错("runtime error: cannot read file \"%s\" - the file does not exist or is not readable\n", &[path.into()]);
        let ep = self.错误流();
        self.调(self.fputc(), &[self.i32t().const_int(10, false).into(), ep.into()], "");
        self.调(self.exit(), &[self.i32t().const_int(1, false).into()], "");
        self.b.build_return(None).unwrap();

        self.b.position_at_end(b_open);
        let fseek = self.函数(
            "fseek",
            Some(self.i32t().into()),
            &[self.p(), self.i64t().into(), self.i32t().into()],
            false,
        );
        let ftell = self.函数("ftell", Some(self.i64t().into()), &[self.p()], false);
        let fread = self.函数(
            "fread",
            Some(self.i64t().into()),
            &[self.p(), self.i64t().into(), self.i64t().into(), self.p()],
            false,
        );
        let fclose = self.函数("fclose", Some(self.i32t().into()), &[self.p()], false);
        self.调(fseek, &[fp.into(), self.i64t().const_zero().into(), self.i32t().const_int(2, false).into()], "");
        let n0 = self.调(ftell, &[fp.into()], "n").unwrap().into_int_value();
        self.调(fseek, &[fp.into(), self.i64t().const_zero().into(), self.i32t().const_zero().into()], "");
        let neg = self.b.build_int_compare(IntPredicate::SLT, n0, self.i64t().const_zero(), "neg").unwrap();
        let n = self
            .b
            .build_select(neg, self.i64t().const_zero(), n0, "nz")
            .unwrap()
            .into_int_value();
        let sz = self.b.build_int_add(n, self.i64t().const_int(1, false), "sz").unwrap();
        let bp = self.调(self.malloc(), &[sz.into()], "b").unwrap().into_pointer_value();
        let b_alloc = self.ctx.append_basic_block(f, "ra");
        let b_oom = self.ctx.append_basic_block(f, "ro");
        self.b.build_conditional_branch(
            self.b.build_is_null(bp, "bn").unwrap(),
            b_oom,
            b_alloc,
        ).unwrap();
        self.b.position_at_end(b_oom);
        let m1 = self.串("out of memory - the program requested more than the available memory");
        let m2 = self.串("reduce the data size, or use a more memory-efficient approach");
        self.调(
            self.die_nc_fn(),
            &[self.i32t().const_int(6, false).into(), m1.into(), m2.into()],
            "",
        );
        self.b.build_return(None).unwrap();

        self.b.position_at_end(b_alloc);
        let got = self
            .调(
                fread,
                &[bp.into(), self.i64t().const_int(1, false).into(), n.into(), fp.into()],
                "got",
            )
            .unwrap()
            .into_int_value();
        let zp = unsafe { self.b.build_gep(self.i8t(), bp, &[got], "ze") }.unwrap();
        self.b.build_store(zp, self.i8t().const_zero()).unwrap();
        self.调(fclose, &[fp.into()], "");
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, bp, 0, "r0").unwrap().into_struct_value();
        sv = self.b.build_insert_value(sv, got, 1, "r1").unwrap().into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn save_file(&self) {
        let f = self.函数("prion_save_file", Some(self.i64t().into()), &[self.p(), self.p()], false);
        if !self.入口(f) {
            return;
        }
        let inp = f.get_nth_param(0).unwrap().into_pointer_value();
        let text = f.get_nth_param(1).unwrap().into_pointer_value();
        let st = self.串型();
        let b_in = self.ctx.append_basic_block(f, "sin");
        let b_null = self.ctx.append_basic_block(f, "snp");
        let b_go = self.ctx.append_basic_block(f, "sgo");
        self.b.build_conditional_branch(
            self.b.build_is_null(inp, "inn").unwrap(),
            b_null,
            b_in,
        ).unwrap();
        self.b.position_at_end(b_in);
        let pp = self.b.build_struct_gep(st, inp, 0, "pp").unwrap();
        let p = self.b.build_load(self.p(), pp, "p").unwrap().into_pointer_value();
        let e1 = self.串("");
        let path0 = self
            .b
            .build_select(self.b.build_is_null(p, "pn").unwrap(), e1, p, "psel")
            .unwrap()
            .into_pointer_value();
        self.b.build_unconditional_branch(b_go).unwrap();
        self.b.position_at_end(b_null);
        let e2 = self.串("");
        self.b.build_unconditional_branch(b_go).unwrap();
        self.b.position_at_end(b_go);
        let path = self.b.build_phi(self.p(), "sp").unwrap();
        path.add_incoming(&[(&path0, b_in), (&e2, b_null)]);
        let path = path.as_basic_value().into_pointer_value();

        let wb = self.函数("prion_fopen_wb", Some(self.p()), &[self.p()], false);
        let fp = self.调(wb, &[path.into()], "fp").unwrap().into_pointer_value();
        let b_open = self.ctx.append_basic_block(f, "sok");
        let b_fail = self.ctx.append_basic_block(f, "sf");
        self.b.build_conditional_branch(
            self.b.build_is_null(fp, "fpn").unwrap(),
            b_fail,
            b_open,
        ).unwrap();

        self.b.position_at_end(b_fail);
        self.报错("runtime error: cannot write file \"%s\" - the directory does not exist or is not writable\n", &[path.into()]);
        let ep = self.错误流();
        self.调(self.fputc(), &[self.i32t().const_int(10, false).into(), ep.into()], "");
        self.调(self.exit(), &[self.i32t().const_int(1, false).into()], "");
        self.b.build_return(Some(&self.i64t().const_zero())).unwrap();

        self.b.position_at_end(b_open);
        let fwrite = self.函数(
            "fwrite",
            Some(self.i64t().into()),
            &[self.p(), self.i64t().into(), self.i64t().into(), self.p()],
            false,
        );
        let fclose = self.函数("fclose", Some(self.i32t().into()), &[self.p()], false);
        let b_w = self.ctx.append_basic_block(f, "sw");
        let b_w2 = self.ctx.append_basic_block(f, "sw2");
        let b_end = self.ctx.append_basic_block(f, "se");
        self.b.build_conditional_branch(
            self.b.build_is_null(text, "tn").unwrap(),
            b_end,
            b_w,
        ).unwrap();
        self.b.position_at_end(b_w);
        let tp = self.b.build_struct_gep(st, text, 0, "tp").unwrap();
        let tv = self.b.build_load(self.p(), tp, "tv").unwrap().into_pointer_value();
        let tnp = self.b.build_struct_gep(st, text, 1, "tnp").unwrap();
        let tn = self.b.build_load(self.i64t(), tnp, "tn").unwrap().into_int_value();
        self.b.build_conditional_branch(
            self.b.build_is_null(tv, "tvn").unwrap(),
            b_end,
            b_w2,
        ).unwrap();
        self.b.position_at_end(b_w2);
        self.调(fwrite, &[tv.into(), self.i64t().const_int(1, false).into(), tn.into(), fp.into()], "");
        self.b.build_unconditional_branch(b_end).unwrap();

        self.b.position_at_end(b_end);
        self.调(fclose, &[fp.into()], "");
        self.b.build_return(Some(&self.i64t().const_int(1, false))).unwrap();
    }

    fn 内部tlptr(&self, name: &str) -> GlobalValue<'ctx> {
        if let Some(g) = self.m.get_global(name) {
            return g;
        }
        let g = self.m.add_global(self.pt(), None, name);
        g.set_linkage(Linkage::Internal);
        g.set_initializer(&self.pt().const_null());
        g
    }

    fn 内部tli64(&self, name: &str, init: i64) -> GlobalValue<'ctx> {
        if let Some(g) = self.m.get_global(name) {
            return g;
        }
        let g = self.m.add_global(self.i64t(), None, name);
        g.set_linkage(Linkage::Internal);
        g.set_initializer(&self.i64t().const_int(init as u64, true));
        g
    }

    fn 内部i64(&self, name: &str) -> GlobalValue<'ctx> {
        if let Some(g) = self.m.get_global(name) {
            return g;
        }
        let g = self.m.add_global(self.i64t(), None, name);
        g.set_linkage(Linkage::Internal);
        g.set_initializer(&self.i64t().const_zero());
        g
    }

    fn 取串(
        &self,
        f: FunctionValue<'ctx>,
        sp: PointerValue<'ctx>,
    ) -> (PointerValue<'ctx>, IntValue<'ctx>) {
        let st = self.串型();
        let b_in = self.块(f, "gin");
        let b_np = self.块(f, "gnp");
        let b_go = self.块(f, "ggo");
        self.b
            .build_conditional_branch(self.b.build_is_null(sp, "sn").unwrap(), b_np, b_in)
            .unwrap();
        self.b.position_at_end(b_in);
        let pp = self.b.build_struct_gep(st, sp, 0, "gp").unwrap();
        let p0 = self.b.build_load(self.p(), pp, "gv").unwrap().into_pointer_value();
        let nl = self.b.build_struct_gep(st, sp, 1, "gn").unwrap();
        let n0 = self.b.build_load(self.i64t(), nl, "gnv").unwrap().into_int_value();
        let e = self.串("");
        let ps = self
            .b
            .build_select(self.b.build_is_null(p0, "pn").unwrap(), e, p0, "gpsel")
            .unwrap()
            .into_pointer_value();
        self.b.build_unconditional_branch(b_go).unwrap();
        self.b.position_at_end(b_np);
        let e2 = self.串("");
        self.b.build_unconditional_branch(b_go).unwrap();
        self.b.position_at_end(b_go);
        let pv = self.b.build_phi(self.p(), "gs").unwrap();
        pv.add_incoming(&[(&ps, b_in), (&e2, b_np)]);
        let nv = self.b.build_phi(self.i64t(), "gnv2").unwrap();
        nv.add_incoming(&[(&n0, b_in), (&self.i64t().const_zero(), b_np)]);
        (
            pv.as_basic_value().into_pointer_value(),
            nv.as_basic_value().into_int_value(),
        )
    }

    fn 空串返回(&self, sret: PointerValue<'ctx>) {
        let e = self.函数("prion_str_empty", None, &[self.p()], false);
        self.调(e, &[sret.into()], "");
    }

    fn 空描述返回(&self, sret: PointerValue<'ctx>) {
        let st = self.串型();
        let mut sv = st.get_undef();
        sv = self
            .b
            .build_insert_value(sv, self.pt().const_null(), 0, "n0")
            .unwrap()
            .into_struct_value();
        sv = self
            .b
            .build_insert_value(sv, self.i64t().const_zero(), 1, "n1")
            .unwrap()
            .into_struct_value();
        self.b.build_store(sret, sv).unwrap();
    }

    fn memcpy(&self) -> FunctionValue<'ctx> {
        self.函数("memcpy", Some(self.p()), &[self.p(), self.p(), self.i64t().into()], false)
    }

    fn memcmp(&self) -> FunctionValue<'ctx> {
        self.函数("memcmp", Some(self.i32t().into()), &[self.p(), self.p(), self.i64t().into()], false)
    }

    fn 字符串(&self) {
        self.串全局();
        self.str_alloc_fn();
        self.str_empty_fn();
        self.str_free_fn();
        self.str_copy_fn();
        self.utf8_len_fn();
        self.char_len_fn();
        self.byte_len_fn();
        self.cv_bind_fn();
        self.cv_at_fn();
        self.str_die_fn();
        self.concat_n_fn();
        self.concat_fn();
        self.sub_str_fn();
        self.nth_char_fn();
        self.iter_start_fn();
        self.iter_next_fn();
        self.char_code_fn();
        self.code_char_fn();
        self.str_eq_fn();
        self.str_index_fn();
        self.int_of_str_fn();
        self.float_of_str_fn();
        self.str_of_int_fn();
        self.str_of_float_fn();
    }

    fn 串全局(&self) {
        self.内部tlptr("prion_cv_s");
        self.内部tli64("prion_cv_total", -1);
        self.内部tli64("prion_cv_n", 0);
        self.内部tli64("prion_cv_off", 0);
        self.内部ptr("prion_it_s");
        self.内部i64("prion_it_i");
        self.内部i64("prion_it_off");
    }

    fn 内存不够(&self) {
        let m1 = self.串("out of memory - the program requested more than the available memory");
        let m2 = self.串("reduce the data size, or use a more memory-efficient approach");
        self.调(
            self.die_nc_fn(),
            &[self.i32t().const_int(6, false).into(), m1.into(), m2.into()],
            "",
        );
    }

    fn str_alloc_fn(&self) {
        let f = self.函数("prion_str_alloc", Some(self.p()), &[self.i64t().into()], false);
        if !self.入口(f) {
            return;
        }
        let n = f.get_nth_param(0).unwrap().into_int_value();
        let b = self.调(self.malloc(), &[n.into()], "p").unwrap().into_pointer_value();
        let b_ok = self.块(f, "sok");
        let b_oom = self.块(f, "soom");
        self.b
            .build_conditional_branch(self.b.build_is_null(b, "bn").unwrap(), b_oom, b_ok)
            .unwrap();
        self.b.position_at_end(b_oom);
        self.内存不够();
        self.b.build_return(Some(&self.pt().const_null())).unwrap();
        self.b.position_at_end(b_ok);
        self.b.build_return(Some(&b)).unwrap();
    }

    fn str_empty_fn(&self) {
        let f = self.函数("prion_str_empty", None, &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let alloc = self.函数("prion_str_alloc", Some(self.p()), &[self.i64t().into()], false);
        let p = self
            .调(alloc, &[self.i64t().const_int(1, false).into()], "p")
            .unwrap()
            .into_pointer_value();
        self.b.build_store(p, self.i8t().const_zero()).unwrap();
        let st = self.串型();
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, p, 0, "e0").unwrap().into_struct_value();
        sv = self
            .b
            .build_insert_value(sv, self.i64t().const_zero(), 1, "e1")
            .unwrap()
            .into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn str_free_fn(&self) {
        let f = self.函数("prion_str_free", None, &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let p = f.get_nth_param(0).unwrap().into_pointer_value();
        let b_do = self.块(f, "fdo");
        let b_end = self.块(f, "fend");
        self.b
            .build_conditional_branch(self.b.build_is_null(p, "fn").unwrap(), b_end, b_do)
            .unwrap();
        self.b.position_at_end(b_do);
        let gs = self.内部tlptr("prion_cv_s");
        let cur = self.b.build_load(self.p(), gs.as_pointer_value(), "cur").unwrap().into_pointer_value();
        let same = self
            .b
            .build_int_compare(
                IntPredicate::EQ,
                self.b.build_ptr_to_int(cur, self.i64t(), "ci").unwrap(),
                self.b.build_ptr_to_int(p, self.i64t(), "pi").unwrap(),
                "same",
            )
            .unwrap();
        let b_clr = self.块(f, "fclr");
        let b_free = self.块(f, "ffr");
        self.b.build_conditional_branch(same, b_clr, b_free).unwrap();
        self.b.position_at_end(b_clr);
        self.b.build_store(gs.as_pointer_value(), self.pt().const_null()).unwrap();
        let gt = self.内部tli64("prion_cv_total", -1);
        self.b
            .build_store(gt.as_pointer_value(), self.i64t().const_int(u64::MAX, true))
            .unwrap();
        let gn = self.内部tli64("prion_cv_n", 0);
        self.b.build_store(gn.as_pointer_value(), self.i64t().const_zero()).unwrap();
        let go = self.内部tli64("prion_cv_off", 0);
        self.b.build_store(go.as_pointer_value(), self.i64t().const_zero()).unwrap();
        self.b.build_unconditional_branch(b_free).unwrap();
        self.b.position_at_end(b_free);
        self.调(self.free(), &[p.into()], "");
        self.b.build_unconditional_branch(b_end).unwrap();
        self.b.position_at_end(b_end);
        self.b.build_return(None).unwrap();
    }

    fn str_copy_fn(&self) {
        let f = self.函数("prion_str_copy", None, &[self.p(), self.p()], false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let sp = f.get_nth_param(1).unwrap().into_pointer_value();
        let st = self.串型();
        let b_in = self.块(f, "cin");
        let b_np = self.块(f, "cnp");
        let b_cp = self.块(f, "ccp");
        self.b
            .build_conditional_branch(self.b.build_is_null(sp, "sn").unwrap(), b_np, b_in)
            .unwrap();
        self.b.position_at_end(b_in);
        let pp = self.b.build_struct_gep(st, sp, 0, "cp").unwrap();
        let p0 = self.b.build_load(self.p(), pp, "cv").unwrap().into_pointer_value();
        let nl = self.b.build_struct_gep(st, sp, 1, "cn").unwrap();
        let n = self.b.build_load(self.i64t(), nl, "cnv").unwrap().into_int_value();
        self.b
            .build_conditional_branch(self.b.build_is_null(p0, "cpn").unwrap(), b_np, b_cp)
            .unwrap();
        self.b.position_at_end(b_np);
        self.空描述返回(sret);
        self.b.build_return(None).unwrap();
        self.b.position_at_end(b_cp);
        let alloc = self.函数("prion_str_alloc", Some(self.p()), &[self.i64t().into()], false);
        let sz = self.b.build_int_add(n, self.i64t().const_int(1, false), "sz").unwrap();
        let b = self.调(alloc, &[sz.into()], "b").unwrap().into_pointer_value();
        self.调(self.memcpy(), &[b.into(), p0.into(), n.into()], "");
        let zp = unsafe { self.b.build_gep(self.i8t(), b, &[n], "ze") }.unwrap();
        self.b.build_store(zp, self.i8t().const_zero()).unwrap();
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, b, 0, "c0").unwrap().into_struct_value();
        sv = self.b.build_insert_value(sv, n, 1, "c1").unwrap().into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn utf8_len_fn(&self) {
        let f = self.函数("prion_utf8_len", Some(self.i64t().into()), &[self.i8t().into()], false);
        if !self.入口(f) {
            return;
        }
        let c = f.get_nth_param(0).unwrap().into_int_value();
        let z = self.b.build_int_z_extend(c, self.i64t(), "z").unwrap();
        let b1 = self.块(f, "u1");
        let b2c = self.块(f, "u2c");
        let b2 = self.块(f, "u2");
        let b3c = self.块(f, "u3c");
        let b3 = self.块(f, "u3");
        let b4c = self.块(f, "u4c");
        let b4 = self.块(f, "u4");
        let l80 = self
            .b
            .build_int_compare(IntPredicate::ULT, z, self.i64t().const_int(0x80, false), "l80")
            .unwrap();
        self.b.build_conditional_branch(l80, b1, b2c).unwrap();
        self.b.position_at_end(b1);
        self.b.build_return(Some(&self.i64t().const_int(1, false))).unwrap();
        self.b.position_at_end(b2c);
        let a5 = self.b.build_and(z, self.i64t().const_int(0xE0, false), "a5").unwrap();
        let e6 = self.b
            .build_int_compare(IntPredicate::EQ, a5, self.i64t().const_int(0xC0, false), "e6")
            .unwrap();
        self.b.build_conditional_branch(e6, b2, b3c).unwrap();
        self.b.position_at_end(b2);
        self.b.build_return(Some(&self.i64t().const_int(2, false))).unwrap();
        self.b.position_at_end(b3c);
        let a4 = self.b.build_and(z, self.i64t().const_int(0xF0, false), "a4").unwrap();
        let ee = self.b
            .build_int_compare(IntPredicate::EQ, a4, self.i64t().const_int(0xE0, false), "ee")
            .unwrap();
        self.b.build_conditional_branch(ee, b3, b4c).unwrap();
        self.b.position_at_end(b3);
        self.b.build_return(Some(&self.i64t().const_int(3, false))).unwrap();
        self.b.position_at_end(b4c);
        let a3 = self.b.build_and(z, self.i64t().const_int(0xF8, false), "a3").unwrap();
        let ef = self.b
            .build_int_compare(IntPredicate::EQ, a3, self.i64t().const_int(0xF0, false), "ef")
            .unwrap();
        self.b.build_conditional_branch(ef, b4, b1).unwrap();
        self.b.position_at_end(b4);
        self.b.build_return(Some(&self.i64t().const_int(4, false))).unwrap();
    }

    fn char_len_fn(&self) {
        let f = self.函数("prion_str_char_len", Some(self.i64t().into()), &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let sp = f.get_nth_param(0).unwrap().into_pointer_value();
        let (s, n) = self.取串(f, sp);
        let cur = self.b.get_insert_block().unwrap();
        let gs = self.内部tlptr("prion_cv_s");
        let cur_s = self.b.build_load(self.p(), gs.as_pointer_value(), "cs").unwrap().into_pointer_value();
        let same = self
            .b
            .build_int_compare(
                IntPredicate::EQ,
                self.b.build_ptr_to_int(cur_s, self.i64t(), "csi").unwrap(),
                self.b.build_ptr_to_int(s, self.i64t(), "si").unwrap(),
                "csame",
            )
            .unwrap();
        let gt = self.内部tli64("prion_cv_total", -1);
        let tot = self.b.build_load(self.i64t(), gt.as_pointer_value(), "tv").unwrap().into_int_value();
        let has = self.b
            .build_int_compare(IntPredicate::SGE, tot, self.i64t().const_zero(), "has")
            .unwrap();
        let hit = self.b.build_and(same, has, "hit").unwrap();
        let b_scan = self.块(f, "cls");
        let b_hit = self.块(f, "clh");
        let b_loop = self.块(f, "cll");
        let b_body = self.块(f, "clb");
        let b_done = self.块(f, "cld");
        self.b.build_conditional_branch(hit, b_hit, b_scan).unwrap();
        self.b.position_at_end(b_hit);
        self.b.build_return(Some(&tot)).unwrap();
        self.b.position_at_end(b_scan);
        self.b.build_unconditional_branch(b_loop).unwrap();
        self.b.position_at_end(b_loop);
        let k = self.b.build_phi(self.i64t(), "ck").unwrap();
        k.add_incoming(&[(&self.i64t().const_zero(), b_scan)]);
        let cnt = self.b.build_phi(self.i64t(), "cc").unwrap();
        cnt.add_incoming(&[(&self.i64t().const_zero(), b_scan)]);
        let ki = k.as_basic_value().into_int_value();
        let ci = cnt.as_basic_value().into_int_value();
        let lt = self.b.build_int_compare(IntPredicate::SLT, ki, n, "lt").unwrap();
        self.b.build_conditional_branch(lt, b_body, b_done).unwrap();
        self.b.position_at_end(b_body);
        let addr = unsafe { self.b.build_gep(self.i8t(), s, &[ki], "ca") }.unwrap();
        let byte = self.b.build_load(self.i8t(), addr, "cb").unwrap().into_int_value();
        let bz = self.b.build_int_z_extend(byte, self.i64t(), "bz").unwrap();
        let a = self.b.build_and(bz, self.i64t().const_int(0xC0, false), "cb2").unwrap();
        let cont = self.b
            .build_int_compare(IntPredicate::EQ, a, self.i64t().const_int(0x80, false), "cont")
            .unwrap();
        let inc = self
            .b
            .build_select(cont, self.i64t().const_zero(), self.i64t().const_int(1, false), "inc")
            .unwrap()
            .into_int_value();
        let cn = self.b.build_int_add(ci, inc, "cn2").unwrap();
        let kn = self.b.build_int_add(ki, self.i64t().const_int(1, false), "kn").unwrap();
        k.add_incoming(&[(&kn, b_body)]);
        cnt.add_incoming(&[(&cn, b_body)]);
        self.b.build_unconditional_branch(b_loop).unwrap();
        self.b.position_at_end(b_done);
        let b_store = self.块(f, "cst");
        let b_ret = self.块(f, "crt");
        self.b.build_conditional_branch(same, b_store, b_ret).unwrap();
        self.b.position_at_end(b_store);
        self.b.build_store(gt.as_pointer_value(), ci).unwrap();
        self.b.build_unconditional_branch(b_ret).unwrap();
        self.b.position_at_end(b_ret);
        self.b.build_return(Some(&ci)).unwrap();
    }

    fn byte_len_fn(&self) {
        let f = self.函数("prion_str_byte_len", Some(self.i64t().into()), &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let s = f.get_nth_param(0).unwrap().into_pointer_value();
        let e = self.串("");
        let p = self
            .b
            .build_select(self.b.build_is_null(s, "sn").unwrap(), e, s, "psel")
            .unwrap()
            .into_pointer_value();
        let r = self.调(self.strlen(), &[p.into()], "n").unwrap().into_int_value();
        self.b.build_return(Some(&r)).unwrap();
    }

    fn cv_bind_fn(&self) {
        let f = self.函数("prion_cv_bind", None, &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let s = f.get_nth_param(0).unwrap().into_pointer_value();
        let gs = self.内部tlptr("prion_cv_s");
        let cur = self.b.build_load(self.p(), gs.as_pointer_value(), "cs").unwrap().into_pointer_value();
        let same = self
            .b
            .build_int_compare(
                IntPredicate::EQ,
                self.b.build_ptr_to_int(cur, self.i64t(), "ci").unwrap(),
                self.b.build_ptr_to_int(s, self.i64t(), "si").unwrap(),
                "same",
            )
            .unwrap();
        let b_set = self.块(f, "bs");
        let b_end = self.块(f, "be");
        self.b.build_conditional_branch(same, b_end, b_set).unwrap();
        self.b.position_at_end(b_set);
        self.b.build_store(gs.as_pointer_value(), s).unwrap();
        let gt = self.内部tli64("prion_cv_total", -1);
        self.b
            .build_store(gt.as_pointer_value(), self.i64t().const_int(u64::MAX, true))
            .unwrap();
        let gn = self.内部tli64("prion_cv_n", 0);
        self.b.build_store(gn.as_pointer_value(), self.i64t().const_zero()).unwrap();
        let go = self.内部tli64("prion_cv_off", 0);
        self.b.build_store(go.as_pointer_value(), self.i64t().const_zero()).unwrap();
        self.b.build_unconditional_branch(b_end).unwrap();
        self.b.position_at_end(b_end);
        self.b.build_return(None).unwrap();
    }

    fn cv_at_fn(&self) {
        let ps = [self.p(), self.i64t().into(), self.i64t().into()];
        let f = self.函数("prion_cv_at", Some(self.i64t().into()), &ps, false);
        if !self.入口(f) {
            return;
        }
        let sp = f.get_nth_param(0).unwrap().into_pointer_value();
        let len = f.get_nth_param(1).unwrap().into_int_value();
        let n = f.get_nth_param(2).unwrap().into_int_value();
        let e = self.串("");
        let s = self
            .b
            .build_select(self.b.build_is_null(sp, "sn").unwrap(), e, sp, "ssel")
            .unwrap()
            .into_pointer_value();
        let bind = self.函数("prion_cv_bind", None, &[self.p()], false);
        self.调(bind, &[s.into()], "");
        let gn = self.内部tli64("prion_cv_n", 0);
        let go = self.内部tli64("prion_cv_off", 0);
        let b_pos = self.块(f, "cpos");
        let b_zero = self.块(f, "cz");
        let npos = self.b
            .build_int_compare(IntPredicate::SGT, n, self.i64t().const_zero(), "np")
            .unwrap();
        self.b.build_conditional_branch(npos, b_pos, b_zero).unwrap();
        self.b.position_at_end(b_zero);
        self.b.build_return(Some(&self.i64t().const_zero())).unwrap();
        self.b.position_at_end(b_pos);
        let curn = self.b.build_load(self.i64t(), gn.as_pointer_value(), "cn").unwrap().into_int_value();
        let b_find = self.块(f, "cf");
        let b_hit = self.块(f, "ch");
        let same = self.b.build_int_compare(IntPredicate::EQ, n, curn, "same").unwrap();
        self.b.build_conditional_branch(same, b_hit, b_find).unwrap();
        self.b.position_at_end(b_hit);
        let curo = self.b.build_load(self.i64t(), go.as_pointer_value(), "co").unwrap().into_int_value();
        self.b.build_return(Some(&curo)).unwrap();
        self.b.position_at_end(b_find);
        let ahead = self.b
            .build_int_compare(IntPredicate::SGT, n, curn, "ahead")
            .unwrap();
        let posn = self.b
            .build_int_compare(IntPredicate::SGT, curn, self.i64t().const_zero(), "posn")
            .unwrap();
        let cur_o = self.b.build_load(self.i64t(), go.as_pointer_value(), "co2").unwrap().into_int_value();
        let inrange = self.b.build_int_compare(IntPredicate::SLT, cur_o, len, "ir").unwrap();
        let c1 = self.b.build_and(ahead, posn, "c1").unwrap();
        let inc = self.b.build_and(c1, inrange, "inc").unwrap();
        let b_inc = self.块(f, "ci");
        let b_scratch = self.块(f, "cs0");
        self.b.build_conditional_branch(inc, b_inc, b_scratch).unwrap();
        self.b.position_at_end(b_inc);
        let b_il = self.块(f, "cil");
        self.b.build_unconditional_branch(b_il).unwrap();
        self.b.position_at_end(b_il);
        let k0 = self.b.build_phi(self.i64t(), "k0").unwrap();
        k0.add_incoming(&[(&cur_o, b_inc)]);
        let c0 = self.b.build_phi(self.i64t(), "c0").unwrap();
        c0.add_incoming(&[(&curn, b_inc)]);
        let b_sl = self.块(f, "csl");
        self.b.build_unconditional_branch(b_sl).unwrap();
        self.b.position_at_end(b_scratch);
        self.b.build_unconditional_branch(b_sl).unwrap();
        self.b.position_at_end(b_sl);
        let k = self.b.build_phi(self.i64t(), "kk").unwrap();
        k.add_incoming(&[(&k0.as_basic_value(), b_il), (&self.i64t().const_zero(), b_scratch)]);
        let c = self.b.build_phi(self.i64t(), "cc").unwrap();
        c.add_incoming(&[(&c0.as_basic_value(), b_il), (&self.i64t().const_int(1, false), b_scratch)]);
        let b_body = self.块(f, "cbd");
        let b_out = self.块(f, "cout");
        let ki = k.as_basic_value().into_int_value();
        let ci = c.as_basic_value().into_int_value();
        let in1 = self.b.build_int_compare(IntPredicate::SLT, ki, len, "i1").unwrap();
        let in2 = self.b.build_int_compare(IntPredicate::SLT, ci, n, "i2").unwrap();
        let goon = self.b.build_and(in1, in2, "go").unwrap();
        self.b.build_conditional_branch(goon, b_body, b_out).unwrap();
        self.b.position_at_end(b_body);
        let addr = unsafe { self.b.build_gep(self.i8t(), s, &[ki], "ba") }.unwrap();
        let byte = self.b.build_load(self.i8t(), addr, "bb").unwrap().into_int_value();
        let u8 = self.函数("prion_utf8_len", Some(self.i64t().into()), &[self.i8t().into()], false);
        let w = self.调(u8, &[byte.into()], "w").unwrap().into_int_value();
        let kn = self.b.build_int_add(ki, w, "kn").unwrap();
        let cn = self.b.build_int_add(ci, self.i64t().const_int(1, false), "cn2").unwrap();
        k.add_incoming(&[(&kn, b_body)]);
        c.add_incoming(&[(&cn, b_body)]);
        self.b.build_unconditional_branch(b_sl).unwrap();
        self.b.position_at_end(b_out);
        self.b.build_store(gn.as_pointer_value(), n).unwrap();
        self.b.build_store(go.as_pointer_value(), ki).unwrap();
        let over = self.b.build_int_compare(IntPredicate::SGT, ki, len, "over").unwrap();
        let rk = self
            .b
            .build_select(over, len, ki, "rk")
            .unwrap()
            .into_int_value();
        self.b.build_return(Some(&rk)).unwrap();
    }

    fn str_die_fn(&self) {
        let ps = [self.p(), self.i64t().into(), self.i64t().into()];
        let f = self.函数("prion_str_die", None, &ps, false);
        if !self.入口(f) {
            return;
        }
        let what = f.get_nth_param(0).unwrap().into_pointer_value();
        let v = f.get_nth_param(1).unwrap().into_int_value();
        let n = f.get_nth_param(2).unwrap().into_int_value();
        self.报错(
            "runtime error: %s index %lld out of bounds - this container has only %lld position(s); check with len(container) before accessing\n",
            &[what.into(), v.into(), n.into()],
        );
        let at = self.函数("prion_at", None, &[], false);
        self.调(at, &[], "");
        self.调(self.exit(), &[self.i32t().const_int(1, false).into()], "");
        self.b.build_return(None).unwrap();
    }

    fn concat_n_fn(&self) {
        let ps = [self.p(), self.i64t().into(), self.p(), self.i64t().into()];
        let f = self.函数("prion_concat_n", None, &ps, false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let n = f.get_nth_param(1).unwrap().into_int_value();
        let parr = f.get_nth_param(2).unwrap().into_pointer_value();
        let chars0 = f.get_nth_param(3).unwrap().into_int_value();
        let st = self.串型();
        let cur = self.入口块(f);
        let b_loop = self.块(f, "knl");
        let b_body = self.块(f, "knb");
        let b_sum = self.块(f, "kns");
        let b_ascii = self.块(f, "kna");
        let b_fin = self.块(f, "knf");
        self.b.build_unconditional_branch(b_loop).unwrap();

        self.b.position_at_end(b_loop);
        let i = self.b.build_phi(self.i64t(), "ki").unwrap();
        i.add_incoming(&[(&self.i64t().const_zero(), cur)]);
        let tot = self.b.build_phi(self.i64t(), "kt").unwrap();
        tot.add_incoming(&[(&self.i64t().const_int(1, false), cur)]);
        let ii = i.as_basic_value().into_int_value();
        let ti = tot.as_basic_value().into_int_value();
        let lt = self.b.build_int_compare(IntPredicate::SLT, ii, n, "kl").unwrap();
        self.b.build_conditional_branch(lt, b_body, b_sum).unwrap();

        self.b.position_at_end(b_body);
        let eoff = self.b.build_int_mul(ii, self.i64t().const_int(16, false), "eo").unwrap();
        let ep = unsafe { self.b.build_gep(self.i8t(), parr, &[eoff], "ep") }.unwrap();
        let pp = self.b.build_load(self.p(), ep, "pp").unwrap().into_pointer_value();
        let nl = self.b.build_int_add(eoff, self.i64t().const_int(8, false), "nlo").unwrap();
        let np = unsafe { self.b.build_gep(self.i8t(), parr, &[nl], "np") }.unwrap();
        let nn = self.b.build_load(self.i64t(), np, "nn").unwrap().into_int_value();
        let nz = self.b.build_select(
            self.b.build_is_null(pp, "pn").unwrap(),
            self.i64t().const_zero(),
            nn,
            "nz",
        ).unwrap().into_int_value();
        let nt = self.b.build_int_add(ti, nz, "nt").unwrap();
        let ni = self.b.build_int_add(ii, self.i64t().const_int(1, false), "ni").unwrap();
        i.add_incoming(&[(&ni, b_body)]);
        tot.add_incoming(&[(&nt, b_body)]);
        self.b.build_unconditional_branch(b_loop).unwrap();

        self.b.position_at_end(b_sum);
        let alloc = self.函数("prion_str_alloc", Some(self.p()), &[self.i64t().into()], false);
        let r = self.调(alloc, &[ti.into()], "r").unwrap().into_pointer_value();
        let b_wl = self.块(f, "kwl");
        let b_wb = self.块(f, "kwb");
        let b_wdone = self.块(f, "kwd");
        self.b.build_unconditional_branch(b_wl).unwrap();

        self.b.position_at_end(b_wl);
        let w = self.b.build_phi(self.p(), "kw").unwrap();
        w.add_incoming(&[(&r, b_sum)]);
        let j = self.b.build_phi(self.i64t(), "kj").unwrap();
        j.add_incoming(&[(&self.i64t().const_zero(), b_sum)]);
        let ji = j.as_basic_value().into_int_value();
        let wj = w.as_basic_value().into_pointer_value();
        let jl = self.b.build_int_compare(IntPredicate::SLT, ji, n, "jl").unwrap();
        self.b.build_conditional_branch(jl, b_wb, b_wdone).unwrap();

        self.b.position_at_end(b_wb);
        let eo2 = self.b.build_int_mul(ji, self.i64t().const_int(16, false), "eo2").unwrap();
        let ep2 = unsafe { self.b.build_gep(self.i8t(), parr, &[eo2], "ep2") }.unwrap();
        let sp2 = self.b.build_load(self.p(), ep2, "sp2").unwrap().into_pointer_value();
        let no2 = self.b.build_int_add(eo2, self.i64t().const_int(8, false), "no2").unwrap();
        let npp = unsafe { self.b.build_gep(self.i8t(), parr, &[no2], "npp") }.unwrap();
        let nn2 = self.b.build_load(self.i64t(), npp, "nn2").unwrap().into_int_value();
        let kk = self.b.build_select(
            self.b.build_is_null(sp2, "spn").unwrap(),
            self.i64t().const_zero(),
            nn2,
            "kk",
        ).unwrap().into_int_value();
        let b_mc = self.块(f, "kmc");
        let b_mz = self.块(f, "kmz");
        let nonz = self.b.build_int_compare(IntPredicate::SGT, kk, self.i64t().const_zero(), "nz2").unwrap();
        self.b.build_conditional_branch(nonz, b_mc, b_mz).unwrap();
        self.b.position_at_end(b_mc);
        self.调(self.memcpy(), &[wj.into(), sp2.into(), kk.into()], "");
        let w2 = unsafe { self.b.build_gep(self.i8t(), wj, &[kk], "w2") }.unwrap();
        self.b.build_unconditional_branch(b_mz).unwrap();
        self.b.position_at_end(b_mz);
        let wn = self.b.build_phi(self.p(), "kwn").unwrap();
        wn.add_incoming(&[(&w2, b_mc), (&wj, b_wb)]);
        let wni = wn.as_basic_value().into_pointer_value();
        let jn = self.b.build_int_add(ji, self.i64t().const_int(1, false), "jn").unwrap();
        w.add_incoming(&[(&wni, b_mz)]);
        j.add_incoming(&[(&jn, b_mz)]);
        self.b.build_unconditional_branch(b_wl).unwrap();

        self.b.position_at_end(b_wdone);
        let wend = w.as_basic_value().into_pointer_value();
        self.b.build_store(wend, self.i8t().const_zero()).unwrap();
        let lend = self.b.build_int_sub(ti, self.i64t().const_int(1, false), "lend").unwrap();
        let negc = self.b.build_int_compare(IntPredicate::SLT, chars0, self.i64t().const_zero(), "negc").unwrap();
        self.b.build_conditional_branch(negc, b_ascii, b_fin).unwrap();

        self.b.position_at_end(b_ascii);
        let b_al = self.块(f, "kal");
        let b_ab = self.块(f, "kab");
        let b_ad = self.块(f, "kad");
        self.b.build_unconditional_branch(b_al).unwrap();
        self.b.position_at_end(b_al);
        let q = self.b.build_phi(self.i64t(), "kq").unwrap();
        q.add_incoming(&[(&self.i64t().const_zero(), b_ascii)]);
        let hi = self.b.build_phi(self.i64t(), "kh").unwrap();
        hi.add_incoming(&[(&self.i64t().const_zero(), b_ascii)]);
        let qi = q.as_basic_value().into_int_value();
        let hii = hi.as_basic_value().into_int_value();
        let ql = self.b.build_int_compare(IntPredicate::SLT, qi, lend, "ql").unwrap();
        self.b.build_conditional_branch(ql, b_ab, b_ad).unwrap();
        self.b.position_at_end(b_ab);
        let qa = unsafe { self.b.build_gep(self.i8t(), r, &[qi], "qa") }.unwrap();
        let qb = self.b.build_load(self.i8t(), qa, "qb").unwrap().into_int_value();
        let qz = self.b.build_int_z_extend(qb, self.i64t(), "qz").unwrap();
        let qm = self.b.build_and(qz, self.i64t().const_int(0x80, false), "qm").unwrap();
        let hn = self.b.build_or(hii, qm, "hn").unwrap();
        let qn = self.b.build_int_add(qi, self.i64t().const_int(1, false), "qn").unwrap();
        q.add_incoming(&[(&qn, b_ab)]);
        hi.add_incoming(&[(&hn, b_ab)]);
        self.b.build_unconditional_branch(b_al).unwrap();
        self.b.position_at_end(b_ad);
        let ascii = self.b.build_int_compare(IntPredicate::EQ, hii, self.i64t().const_zero(), "asc").unwrap();
        let chars = self
            .b
            .build_select(ascii, lend, chars0, "chars")
            .unwrap()
            .into_int_value();
        self.b.build_unconditional_branch(b_fin).unwrap();

        self.b.position_at_end(b_fin);
        let ch = self.b.build_phi(self.i64t(), "kch").unwrap();
        ch.add_incoming(&[(&chars, b_ad), (&chars0, b_wdone)]);
        let chi = ch.as_basic_value().into_int_value();
        let b_set = self.块(f, "kset");
        let b_done = self.块(f, "kdone");
        let nonneg = self.b.build_int_compare(IntPredicate::SGE, chi, self.i64t().const_zero(), "nng").unwrap();
        self.b.build_conditional_branch(nonneg, b_set, b_done).unwrap();
        self.b.position_at_end(b_set);
        let gs = self.内部tlptr("prion_cv_s");
        self.b.build_store(gs.as_pointer_value(), r).unwrap();
        let gt = self.内部tli64("prion_cv_total", -1);
        let gtv = self.b.build_load(self.i64t(), gt.as_pointer_value(), "gtv").unwrap().into_int_value();
        let gn = self.内部tli64("prion_cv_n", 0);
        let gov = self.内部tli64("prion_cv_off", 0);
        self.b.build_store(gt.as_pointer_value(), chi).unwrap();
        self.b.build_store(gn.as_pointer_value(), self.i64t().const_zero()).unwrap();
        self.b.build_store(gov.as_pointer_value(), self.i64t().const_zero()).unwrap();
        let _ = gtv;
        self.b.build_unconditional_branch(b_done).unwrap();
        self.b.position_at_end(b_done);
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, r, 0, "k0").unwrap().into_struct_value();
        sv = self.b.build_insert_value(sv, lend, 1, "k1").unwrap().into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn concat_fn(&self) {
        let f = self.函数("prion_concat", None, &[self.p(), self.p(), self.p()], false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let a = f.get_nth_param(1).unwrap().into_pointer_value();
        let b = f.get_nth_param(2).unwrap().into_pointer_value();
        let st = self.串型();
        let ty2 = st.array_type(2);
        let zi = self.i64t().const_zero();
        let oi = self.i64t().const_int(1, false);
        let arr = self.b.build_alloca(ty2, "ps").unwrap();
        let cn = self.函数(
            "prion_concat_n",
            None,
            &[self.p(), self.i64t().into(), self.p(), self.i64t().into()],
            false,
        );
        let one = self.块(f, "ca1");
        let zero = self.块(f, "ca0");
        self.b
            .build_conditional_branch(self.b.build_is_null(a, "an").unwrap(), zero, one)
            .unwrap();
        self.b.position_at_end(one);
        let av = self.b.build_load(st, a, "av").unwrap().into_struct_value();
        let s0 = unsafe { self.b.build_gep(ty2, arr, &[zi, zi], "s0") }.unwrap();
        self.b.build_store(s0, av).unwrap();
        let b_after0 = self.块(f, "caa0");
        self.b.build_unconditional_branch(b_after0).unwrap();
        self.b.position_at_end(zero);
        let mut zv = st.get_undef();
        zv = self
            .b
            .build_insert_value(zv, self.pt().const_null(), 0, "z0")
            .unwrap()
            .into_struct_value();
        zv = self
            .b
            .build_insert_value(zv, self.i64t().const_zero(), 1, "z1")
            .unwrap()
            .into_struct_value();
        let z0 = unsafe { self.b.build_gep(ty2, arr, &[zi, zi], "z0p") }.unwrap();
        self.b.build_store(z0, zv).unwrap();
        self.b.build_unconditional_branch(b_after0).unwrap();
        self.b.position_at_end(b_after0);
        let b1 = self.块(f, "cb1");
        let b1z = self.块(f, "cb1z");
        let after1 = self.块(f, "caa1");
        self.b
            .build_conditional_branch(self.b.build_is_null(b, "bn").unwrap(), b1z, b1)
            .unwrap();
        self.b.position_at_end(b1);
        let bv = self.b.build_load(st, b, "bv").unwrap().into_struct_value();
        let s1 = unsafe { self.b.build_gep(ty2, arr, &[zi, oi], "s1") }.unwrap();
        self.b.build_store(s1, bv).unwrap();
        self.b.build_unconditional_branch(after1).unwrap();
        self.b.position_at_end(b1z);
        let mut zv2 = st.get_undef();
        zv2 = self
            .b
            .build_insert_value(zv2, self.pt().const_null(), 0, "y0")
            .unwrap()
            .into_struct_value();
        zv2 = self
            .b
            .build_insert_value(zv2, self.i64t().const_zero(), 1, "y1")
            .unwrap()
            .into_struct_value();
        let z1 = unsafe { self.b.build_gep(ty2, arr, &[zi, oi], "z1p") }.unwrap();
        self.b.build_store(z1, zv2).unwrap();
        self.b.build_unconditional_branch(after1).unwrap();
        self.b.position_at_end(after1);
        self.调(
            cn,
            &[
                sret.into(),
                self.i64t().const_int(2, false).into(),
                arr.into(),
                self.i64t().const_int(u64::MAX, true).into(),
            ],
            "",
        );
        self.b.build_return(None).unwrap();
    }

    fn sub_str_fn(&self) {
        let ps = [self.p(), self.p(), self.i64t().into(), self.i64t().into()];
        let f = self.函数("prion_sub_str", None, &ps, false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let sp = f.get_nth_param(1).unwrap().into_pointer_value();
        let a0 = f.get_nth_param(2).unwrap().into_int_value();
        let b0 = f.get_nth_param(3).unwrap().into_int_value();
        let (s, n) = self.取串(f, sp);
        let one = self.i64t().const_int(1, false);
        let a1 = self.b.build_int_compare(IntPredicate::SLT, a0, one, "al").unwrap();
        let a = self.b.build_select(a1, one, a0, "av").unwrap().into_int_value();
        let b_gt = self.b.build_int_compare(IntPredicate::SGT, b0, n, "bg").unwrap();
        let b = self.b.build_select(b_gt, n, b0, "bv").unwrap().into_int_value();
        let b_lt = self.b.build_int_compare(IntPredicate::SLT, b, a, "bl").unwrap();
        let b_empty = self.块(f, "sse");
        let b_go = self.块(f, "ssg");
        self.b.build_conditional_branch(b_lt, b_empty, b_go).unwrap();
        self.b.position_at_end(b_empty);
        let e = self.函数("prion_str_empty", None, &[self.p()], false);
        self.调(e, &[sret.into()], "");
        self.b.build_return(None).unwrap();
        self.b.position_at_end(b_go);
        let len0 = self.b.build_int_sub(b, a, "ln0").unwrap();
        let len = self.b.build_int_add(len0, one, "ln").unwrap();
        let len1 = self.b.build_int_add(len, one, "l1").unwrap();
        let alloc = self.函数("prion_str_alloc", Some(self.p()), &[self.i64t().into()], false);
        let r = self.调(alloc, &[len1.into()], "r").unwrap().into_pointer_value();
        let off = self.b.build_int_sub(a, one, "off").unwrap();
        let src = unsafe { self.b.build_gep(self.i8t(), s, &[off], "src") }.unwrap();
        self.调(self.memcpy(), &[r.into(), src.into(), len.into()], "");
        let zp = unsafe { self.b.build_gep(self.i8t(), r, &[len], "zp") }.unwrap();
        self.b.build_store(zp, self.i8t().const_zero()).unwrap();
        let st = self.串型();
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, r, 0, "s0").unwrap().into_struct_value();
        sv = self.b.build_insert_value(sv, len, 1, "s1").unwrap().into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn nth_char_fn(&self) {
        let ps = [self.p(), self.p(), self.i64t().into()];
        let f = self.函数("prion_nth_char", None, &ps, false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let sp = f.get_nth_param(1).unwrap().into_pointer_value();
        let n = f.get_nth_param(2).unwrap().into_int_value();
        let (s, len) = self.取串(f, sp);
        let cl = self.函数("prion_str_char_len", Some(self.i64t().into()), &[self.p()], false);
        let total = self.调(cl, &[sp.into()], "tot").unwrap().into_int_value();
        let one = self.i64t().const_int(1, false);
        let lo = self.b.build_int_compare(IntPredicate::SLT, n, one, "lo").unwrap();
        let hi = self.b.build_int_compare(IntPredicate::SGT, n, total, "hi").unwrap();
        let bad = self.b.build_or(lo, hi, "bad").unwrap();
        let b_empty = self.块(f, "ne");
        let b_go = self.块(f, "ng");
        self.b.build_conditional_branch(bad, b_empty, b_go).unwrap();
        self.b.position_at_end(b_empty);
        self.空串返回(sret);
        self.b.build_return(None).unwrap();
        self.b.position_at_end(b_go);
        let ca = self.函数(
            "prion_cv_at",
            Some(self.i64t().into()),
            &[self.p(), self.i64t().into(), self.i64t().into()],
            false,
        );
        let k = self.调(ca, &[s.into(), len.into(), n.into()], "k").unwrap().into_int_value();
        let ov = self.b.build_int_compare(IntPredicate::SGE, k, len, "ov").unwrap();
        let b_e2 = self.块(f, "ne2");
        let b_cp = self.块(f, "np");
        self.b.build_conditional_branch(ov, b_e2, b_cp).unwrap();
        self.b.position_at_end(b_e2);
        self.空串返回(sret);
        self.b.build_return(None).unwrap();
        self.b.position_at_end(b_cp);
        let ka = unsafe { self.b.build_gep(self.i8t(), s, &[k], "ka") }.unwrap();
        let kb = self.b.build_load(self.i8t(), ka, "kb").unwrap().into_int_value();
        let u8 = self.函数("prion_utf8_len", Some(self.i64t().into()), &[self.i8t().into()], false);
        let w0 = self.调(u8, &[kb.into()], "w").unwrap().into_int_value();
        let sum = self.b.build_int_add(k, w0, "sm").unwrap();
        let ov2 = self.b.build_int_compare(IntPredicate::SGT, sum, len, "ov2").unwrap();
        let wl = self.b.build_int_sub(len, k, "wl").unwrap();
        let w = self.b.build_select(ov2, wl, w0, "wv").unwrap().into_int_value();
        let w1 = self.b.build_int_add(w, one, "w1").unwrap();
        let alloc = self.函数("prion_str_alloc", Some(self.p()), &[self.i64t().into()], false);
        let b = self.调(alloc, &[w1.into()], "b").unwrap().into_pointer_value();
        self.调(self.memcpy(), &[b.into(), ka.into(), w.into()], "");
        let zp = unsafe { self.b.build_gep(self.i8t(), b, &[w], "zp") }.unwrap();
        self.b.build_store(zp, self.i8t().const_zero()).unwrap();
        let st = self.串型();
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, b, 0, "p0").unwrap().into_struct_value();
        sv = self.b.build_insert_value(sv, w, 1, "p1").unwrap().into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn 入口块(&self, f: FunctionValue<'ctx>) -> inkwell::basic_block::BasicBlock<'ctx> {
        f.get_first_basic_block().unwrap()
    }

    fn iter_start_fn(&self) {
        let f = self.函数("prion_str_iter_start", None, &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let (s, _) = {
            let sp = f.get_nth_param(0).unwrap().into_pointer_value();
            self.取串(f, sp)
        };
        let gs = self.内部ptr("prion_it_s");
        self.b.build_store(gs.as_pointer_value(), s).unwrap();
        let gi = self.内部i64("prion_it_i");
        self.b.build_store(gi.as_pointer_value(), self.i64t().const_zero()).unwrap();
        let go = self.内部i64("prion_it_off");
        self.b.build_store(go.as_pointer_value(), self.i64t().const_zero()).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn iter_next_fn(&self) {
        let ps = [self.p(), self.p(), self.i64t().into()];
        let f = self.函数("prion_str_iter_next", None, &ps, false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let sp = f.get_nth_param(1).unwrap().into_pointer_value();
        let n = f.get_nth_param(2).unwrap().into_int_value();
        let (s, len) = self.取串(f, sp);
        let gs = self.内部ptr("prion_it_s");
        let gi = self.内部i64("prion_it_i");
        let go = self.内部i64("prion_it_off");
        let cur_s = self.b.build_load(self.p(), gs.as_pointer_value(), "is").unwrap().into_pointer_value();
        let cur_i = self.b.build_load(self.i64t(), gi.as_pointer_value(), "ii").unwrap().into_int_value();
        let cur_o = self.b.build_load(self.i64t(), go.as_pointer_value(), "io").unwrap().into_int_value();
        let same = self.b
            .build_int_compare(
                IntPredicate::EQ,
                self.b.build_ptr_to_int(cur_s, self.i64t(), "si").unwrap(),
                self.b.build_ptr_to_int(s, self.i64t(), "ti").unwrap(),
                "isame",
            )
            .unwrap();
        let one = self.i64t().const_int(1, false);
        let pos = self.b.build_int_compare(IntPredicate::SGT, cur_i, self.i64t().const_zero(), "pos").unwrap();
        let nxt = self.b.build_int_add(cur_i, one, "nxt").unwrap();
        let isn = self.b.build_int_compare(IntPredicate::EQ, n, nxt, "isn").unwrap();
        let inr = self.b.build_int_compare(IntPredicate::SLT, cur_o, len, "inr").unwrap();
        let c1 = self.b.build_and(same, pos, "c1").unwrap();
        let c2 = self.b.build_and(c1, isn, "c2").unwrap();
        let nextok = self.b.build_and(c2, inr, "nok").unwrap();
        let eqn = self.b.build_int_compare(IntPredicate::EQ, n, cur_i, "eqn").unwrap();
        let sameok = self.b.build_and(same, eqn, "sok").unwrap();
        let b_fast = self.块(f, "itf");
        let b_same = self.块(f, "its");
        let b_kl = self.块(f, "itl");
        let b_kb = self.块(f, "itb");
        let b_kd = self.块(f, "itd");
        self.b.build_conditional_branch(nextok, b_fast, b_same).unwrap();
        self.b.position_at_end(b_fast);
        let ca = unsafe { self.b.build_gep(self.i8t(), s, &[cur_o], "fa") }.unwrap();
        let cb = self.b.build_load(self.i8t(), ca, "fb").unwrap().into_int_value();
        let u8 = self.函数("prion_utf8_len", Some(self.i64t().into()), &[self.i8t().into()], false);
        let fw = self.调(u8, &[cb.into()], "fw").unwrap().into_int_value();
        let fk = self.b.build_int_add(cur_o, fw, "fk").unwrap();
        self.b.build_unconditional_branch(b_kd).unwrap();
        self.b.position_at_end(b_same);
        self.b.build_conditional_branch(sameok, b_kd, b_kl).unwrap();
        self.b.position_at_end(b_kl);
        let sk = self.b.build_phi(self.i64t(), "sk").unwrap();
        sk.add_incoming(&[(&self.i64t().const_zero(), b_same)]);
        let sc = self.b.build_phi(self.i64t(), "sc").unwrap();
        sc.add_incoming(&[(&self.i64t().const_zero(), b_same)]);
        let ski = sk.as_basic_value().into_int_value();
        let sci = sc.as_basic_value().into_int_value();
        let skl = self.b.build_int_compare(IntPredicate::SLT, ski, len, "skl").unwrap();
        self.b.build_conditional_branch(skl, b_kb, b_kd).unwrap();
        self.b.position_at_end(b_kb);
        let scn = self.b.build_int_add(sci, one, "scn").unwrap();
        let done = self.b.build_int_compare(IntPredicate::EQ, scn, n, "sd").unwrap();
        let b_break = self.块(f, "itk");
        let b_cont = self.块(f, "itn");
        self.b.build_conditional_branch(done, b_break, b_cont).unwrap();
        self.b.position_at_end(b_break);
        self.b.build_unconditional_branch(b_kd).unwrap();
        self.b.position_at_end(b_cont);
        let aa = unsafe { self.b.build_gep(self.i8t(), s, &[ski], "aa") }.unwrap();
        let ab = self.b.build_load(self.i8t(), aa, "ab").unwrap().into_int_value();
        let w2 = self.调(u8, &[ab.into()], "w2").unwrap().into_int_value();
        let kn = self.b.build_int_add(ski, w2, "kn").unwrap();
        sk.add_incoming(&[(&kn, b_cont)]);
        sc.add_incoming(&[(&scn, b_cont)]);
        self.b.build_unconditional_branch(b_kl).unwrap();
        self.b.position_at_end(b_kd);
        let kfin = self.b.build_phi(self.i64t(), "kf").unwrap();
        kfin.add_incoming(&[(&fk, b_fast)]);
        kfin.add_incoming(&[(&cur_o, b_same)]);
        kfin.add_incoming(&[(&ski, b_kl)]);
        kfin.add_incoming(&[(&ski, b_break)]);
        let kfi = kfin.as_basic_value().into_int_value();
        let b_empty = self.块(f, "ite");
        let b_ok = self.块(f, "ito");
        self.b.build_conditional_branch(
            self.b.build_int_compare(IntPredicate::SGE, kfi, len, "end").unwrap(),
            b_empty,
            b_ok,
        ).unwrap();
        self.b.position_at_end(b_empty);
        self.空串返回(sret);
        self.b.build_return(None).unwrap();
        self.b.position_at_end(b_ok);
        let ka = unsafe { self.b.build_gep(self.i8t(), s, &[kfi], "ka") }.unwrap();
        let kb = self.b.build_load(self.i8t(), ka, "kb").unwrap().into_int_value();
        let w0 = self.调(u8, &[kb.into()], "w0").unwrap().into_int_value();
        let sum = self.b.build_int_add(kfi, w0, "sm").unwrap();
        let ov = self.b.build_int_compare(IntPredicate::SGT, sum, len, "ov").unwrap();
        let wl = self.b.build_int_sub(len, kfi, "wl").unwrap();
        let w = self.b.build_select(ov, wl, w0, "wv").unwrap().into_int_value();
        let w1 = self.b.build_int_add(w, one, "w1").unwrap();
        let alloc = self.函数("prion_str_alloc", Some(self.p()), &[self.i64t().into()], false);
        let b = self.调(alloc, &[w1.into()], "b").unwrap().into_pointer_value();
        self.调(self.memcpy(), &[b.into(), ka.into(), w.into()], "");
        let zp = unsafe { self.b.build_gep(self.i8t(), b, &[w], "zp") }.unwrap();
        self.b.build_store(zp, self.i8t().const_zero()).unwrap();
        self.b.build_store(gs.as_pointer_value(), s).unwrap();
        self.b.build_store(gi.as_pointer_value(), n).unwrap();
        self.b.build_store(go.as_pointer_value(), kfi).unwrap();
        let st = self.串型();
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, b, 0, "x0").unwrap().into_struct_value();
        sv = self.b.build_insert_value(sv, w, 1, "x1").unwrap().into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn char_code_fn(&self) {
        let f = self.函数("prion_char_code", Some(self.i64t().into()), &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let sp = f.get_nth_param(0).unwrap().into_pointer_value();
        let (s, _) = self.取串(f, sp);
        let b0p = unsafe { self.b.build_gep(self.i8t(), s, &[self.i64t().const_zero()], "b0p") }.unwrap();
        let b0 = self.b.build_load(self.i8t(), b0p, "b0").unwrap().into_int_value();
        let c0 = self.b.build_int_z_extend(b0, self.i64t(), "c0").unwrap();
        let p1 = unsafe { self.b.build_gep(self.i8t(), s, &[self.i64t().const_int(1, false)], "p1") }.unwrap();
        let b1 = self.b.build_load(self.i8t(), p1, "b1").unwrap().into_int_value();
        let c1 = self.b.build_int_z_extend(b1, self.i64t(), "c1").unwrap();
        let p2 = unsafe { self.b.build_gep(self.i8t(), s, &[self.i64t().const_int(2, false)], "p2") }.unwrap();
        let b2 = self.b.build_load(self.i8t(), p2, "b2").unwrap().into_int_value();
        let c2 = self.b.build_int_z_extend(b2, self.i64t(), "c2").unwrap();
        let p3 = unsafe { self.b.build_gep(self.i8t(), s, &[self.i64t().const_int(3, false)], "p3") }.unwrap();
        let b3 = self.b.build_load(self.i8t(), p3, "b3").unwrap().into_int_value();
        let c3 = self.b.build_int_z_extend(b3, self.i64t(), "c3").unwrap();
        let z = self.i64t().const_zero();
        let m3f = self.i64t().const_int(0x3F, false);
        let b_neg = self.块(f, "cc0");
        let b_ascii = self.块(f, "cca");
        let b_c2 = self.块(f, "ccc2");
        let b_c3 = self.块(f, "ccc3");
        let b_c4 = self.块(f, "ccc4");
        let b_last = self.块(f, "ccl");
        self.b.build_conditional_branch(
            self.b.build_int_compare(IntPredicate::EQ, c0, z, "e0").unwrap(),
            b_neg,
            b_ascii,
        ).unwrap();
        self.b.position_at_end(b_neg);
        self.b.build_return(Some(&self.i64t().const_int(u64::MAX, true))).unwrap();
        self.b.position_at_end(b_ascii);
        self.b.build_conditional_branch(
            self.b.build_int_compare(IntPredicate::ULT, c0, self.i64t().const_int(0x80, false), "a").unwrap(),
            b_last,
            b_c2,
        ).unwrap();
        self.b.position_at_end(b_c2);
        let a2 = self.b.build_and(c0, self.i64t().const_int(0xE0, false), "a2").unwrap();
        let e2 = self.b.build_int_compare(IntPredicate::EQ, a2, self.i64t().const_int(0xC0, false), "e2").unwrap();
        let b_do2 = self.块(f, "cd2");
        self.b.build_conditional_branch(e2, b_do2, b_c3).unwrap();
        self.b.position_at_end(b_do2);
        let t1 = self.b.build_and(c0, self.i64t().const_int(0x1F, false), "t1").unwrap();
        let t2 = self.b.build_left_shift(t1, self.i64t().const_int(6, false), "t2").unwrap();
        let t3 = self.b.build_and(c1, m3f, "t3").unwrap();
        let r2 = self.b.build_or(t2, t3, "r2").unwrap();
        self.b.build_return(Some(&r2)).unwrap();
        self.b.position_at_end(b_c3);
        let a3 = self.b.build_and(c0, self.i64t().const_int(0xF0, false), "a3").unwrap();
        let e3 = self.b.build_int_compare(IntPredicate::EQ, a3, self.i64t().const_int(0xE0, false), "e3").unwrap();
        let b_do3 = self.块(f, "cd3");
        self.b.build_conditional_branch(e3, b_do3, b_c4).unwrap();
        self.b.position_at_end(b_do3);
        let u1 = self.b.build_and(c0, self.i64t().const_int(0x0F, false), "u1").unwrap();
        let u2 = self.b.build_left_shift(u1, self.i64t().const_int(12, false), "u2").unwrap();
        let u3 = self.b.build_and(c1, m3f, "u3").unwrap();
        let u4 = self.b.build_left_shift(u3, self.i64t().const_int(6, false), "u4").unwrap();
        let u5 = self.b.build_or(u2, u4, "u5").unwrap();
        let u6 = self.b.build_and(c2, m3f, "u6").unwrap();
        let r3 = self.b.build_or(u5, u6, "r3").unwrap();
        self.b.build_return(Some(&r3)).unwrap();
        self.b.position_at_end(b_c4);
        let v1 = self.b.build_and(c0, self.i64t().const_int(0xF8, false), "v1").unwrap();
        let v2 = self.b.build_int_compare(IntPredicate::EQ, v1, self.i64t().const_int(0xF0, false), "v2").unwrap();
        let b_do4 = self.块(f, "cd4");
        self.b.build_conditional_branch(v2, b_do4, b_last).unwrap();
        self.b.position_at_end(b_do4);
        let w1 = self.b.build_and(c0, self.i64t().const_int(0x07, false), "w1").unwrap();
        let w2 = self.b.build_left_shift(w1, self.i64t().const_int(18, false), "w2").unwrap();
        let w3 = self.b.build_and(c1, m3f, "w3").unwrap();
        let w4 = self.b.build_left_shift(w3, self.i64t().const_int(12, false), "w4").unwrap();
        let w5 = self.b.build_or(w2, w4, "w5").unwrap();
        let w6 = self.b.build_and(c2, m3f, "w6").unwrap();
        let w7 = self.b.build_left_shift(w6, self.i64t().const_int(6, false), "w7").unwrap();
        let w8 = self.b.build_or(w5, w7, "w8").unwrap();
        let w9 = self.b.build_and(c3, m3f, "w9").unwrap();
        let r4 = self.b.build_or(w8, w9, "r4").unwrap();
        self.b.build_return(Some(&r4)).unwrap();
        self.b.position_at_end(b_last);
        self.b.build_return(Some(&c0)).unwrap();
    }

    fn code_char_fn(&self) {
        let f = self.函数("prion_code_char", None, &[self.p(), self.i64t().into()], false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let cp = f.get_nth_param(1).unwrap().into_int_value();
        let alloc = self.函数("prion_str_alloc", Some(self.p()), &[self.i64t().into()], false);
        let r = self.调(alloc, &[self.i64t().const_int(5, false).into()], "r")
            .unwrap()
            .into_pointer_value();
        let bad1 = self.b
            .build_int_compare(IntPredicate::SLT, cp, self.i64t().const_zero(), "b1")
            .unwrap();
        let bad2 = self.b
            .build_int_compare(IntPredicate::SGT, cp, self.i64t().const_int(0x10FFFF, false), "b2")
            .unwrap();
        let sur1 = self.b
            .build_int_compare(IntPredicate::SGE, cp, self.i64t().const_int(0xD800, false), "s1")
            .unwrap();
        let sur2 = self.b
            .build_int_compare(IntPredicate::SLE, cp, self.i64t().const_int(0xDFFF, false), "s2")
            .unwrap();
        let sur = self.b.build_and(sur1, sur2, "sur").unwrap();
        let b1o = self.b.build_or(bad1, bad2, "bo").unwrap();
        let bad = self.b.build_or(b1o, sur, "bad").unwrap();
        let b_zero = self.块(f, "ccz");
        let b_a = self.块(f, "cca");
        self.b.build_conditional_branch(bad, b_zero, b_a).unwrap();
        self.b.position_at_end(b_zero);
        self.b.build_store(r, self.i8t().const_zero()).unwrap();
        let st = self.串型();
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, r, 0, "z0").unwrap().into_struct_value();
        sv = self
            .b
            .build_insert_value(sv, self.i64t().const_zero(), 1, "z1")
            .unwrap()
            .into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
        self.b.position_at_end(b_a);
        let b2 = self.块(f, "c2");
        let b3 = self.块(f, "c3");
        let b4 = self.块(f, "c4");
        let b4v = self.块(f, "c4v");
        let lim1 = self.b
            .build_int_compare(IntPredicate::ULT, cp, self.i64t().const_int(0x80, false), "l1")
            .unwrap();
        self.b.build_conditional_branch(lim1, b2, b3).unwrap();
        self.b.position_at_end(b2);
        let t = self.b.build_int_truncate(cp, self.i8t(), "t").unwrap();
        self.b.build_store(r, t).unwrap();
        let n2 = unsafe { self.b.build_gep(self.i8t(), r, &[self.i64t().const_int(1, false)], "n2") }.unwrap();
        self.b.build_unconditional_branch(b4v).unwrap();
        self.b.position_at_end(b3);
        let lim2 = self.b
            .build_int_compare(IntPredicate::ULT, cp, self.i64t().const_int(0x800, false), "l2")
            .unwrap();
        let b3do = self.块(f, "c3d");
        self.b.build_conditional_branch(lim2, b3do, b4).unwrap();
        self.b.position_at_end(b3do);
        let q1 = self.b.build_right_shift(cp, self.i64t().const_int(6, false), false, "q1").unwrap();
        let q2 = self.b.build_or(q1, self.i64t().const_int(0xC0, false), "q2").unwrap();
        let q2t = self.b.build_int_truncate(q2, self.i8t(), "q2t").unwrap();
        self.b.build_store(r, q2t).unwrap();
        let r1 = unsafe { self.b.build_gep(self.i8t(), r, &[self.i64t().const_int(1, false)], "r1") }.unwrap();
        let q3 = self.b.build_and(cp, self.i64t().const_int(0x3F, false), "q3").unwrap();
        let q4 = self.b.build_or(q3, self.i64t().const_int(0x80, false), "q4").unwrap();
        let q4t = self.b.build_int_truncate(q4, self.i8t(), "q4t").unwrap();
        self.b.build_store(r1, q4t).unwrap();
        let n3 = unsafe { self.b.build_gep(self.i8t(), r, &[self.i64t().const_int(2, false)], "n3") }.unwrap();
        self.b.build_unconditional_branch(b4v).unwrap();
        self.b.position_at_end(b4);
        let lim3 = self.b
            .build_int_compare(IntPredicate::ULT, cp, self.i64t().const_int(0x10000, false), "l3")
            .unwrap();
        let b4do = self.块(f, "c4d");
        let b4do5 = self.块(f, "c5d");
        self.b.build_conditional_branch(lim3, b4do, b4do5).unwrap();
        self.b.position_at_end(b4do);
        let s1 = self.b.build_right_shift(cp, self.i64t().const_int(12, false), false, "s1").unwrap();
        let s2 = self.b.build_or(s1, self.i64t().const_int(0xE0, false), "s2").unwrap();
        self.b.build_store(r, self.b.build_int_truncate(s2, self.i8t(), "s2t").unwrap()).unwrap();
        let m1 = unsafe { self.b.build_gep(self.i8t(), r, &[self.i64t().const_int(1, false)], "m1") }.unwrap();
        let s3 = self.b.build_right_shift(cp, self.i64t().const_int(6, false), false, "s3").unwrap();
        let s4 = self.b.build_and(s3, self.i64t().const_int(0x3F, false), "s4").unwrap();
        let s5 = self.b.build_or(s4, self.i64t().const_int(0x80, false), "s5").unwrap();
        self.b.build_store(m1, self.b.build_int_truncate(s5, self.i8t(), "s5t").unwrap()).unwrap();
        let m2 = unsafe { self.b.build_gep(self.i8t(), r, &[self.i64t().const_int(2, false)], "m2") }.unwrap();
        let s6 = self.b.build_and(cp, self.i64t().const_int(0x3F, false), "s6").unwrap();
        let s7 = self.b.build_or(s6, self.i64t().const_int(0x80, false), "s7").unwrap();
        self.b.build_store(m2, self.b.build_int_truncate(s7, self.i8t(), "s7t").unwrap()).unwrap();
        let n4 = unsafe { self.b.build_gep(self.i8t(), r, &[self.i64t().const_int(3, false)], "n4") }.unwrap();
        self.b.build_unconditional_branch(b4v).unwrap();
        self.b.position_at_end(b4do5);
        let t1 = self.b.build_right_shift(cp, self.i64t().const_int(18, false), false, "t1").unwrap();
        let t2 = self.b.build_or(t1, self.i64t().const_int(0xF0, false), "t2").unwrap();
        self.b.build_store(r, self.b.build_int_truncate(t2, self.i8t(), "t2t").unwrap()).unwrap();
        let u1 = unsafe { self.b.build_gep(self.i8t(), r, &[self.i64t().const_int(1, false)], "u1") }.unwrap();
        let u2 = self.b.build_right_shift(cp, self.i64t().const_int(12, false), false, "u2").unwrap();
        let u3 = self.b.build_and(u2, self.i64t().const_int(0x3F, false), "u3").unwrap();
        let u4 = self.b.build_or(u3, self.i64t().const_int(0x80, false), "u4").unwrap();
        self.b.build_store(u1, self.b.build_int_truncate(u4, self.i8t(), "u4t").unwrap()).unwrap();
        let u5 = unsafe { self.b.build_gep(self.i8t(), r, &[self.i64t().const_int(2, false)], "u5") }.unwrap();
        let u6 = self.b.build_right_shift(cp, self.i64t().const_int(6, false), false, "u6").unwrap();
        let u7 = self.b.build_and(u6, self.i64t().const_int(0x3F, false), "u7").unwrap();
        let u8v = self.b.build_or(u7, self.i64t().const_int(0x80, false), "u8").unwrap();
        self.b.build_store(u5, self.b.build_int_truncate(u8v, self.i8t(), "u8t").unwrap()).unwrap();
        let u9 = unsafe { self.b.build_gep(self.i8t(), r, &[self.i64t().const_int(3, false)], "u9") }.unwrap();
        let v1 = self.b.build_and(cp, self.i64t().const_int(0x3F, false), "v1").unwrap();
        let v2 = self.b.build_or(v1, self.i64t().const_int(0x80, false), "v2").unwrap();
        self.b.build_store(u9, self.b.build_int_truncate(v2, self.i8t(), "v2t").unwrap()).unwrap();
        let n5 = unsafe { self.b.build_gep(self.i8t(), r, &[self.i64t().const_int(4, false)], "n5") }.unwrap();
        self.b.build_unconditional_branch(b4v).unwrap();
        self.b.position_at_end(b4v);
        let np = self.b.build_phi(self.p(), "np").unwrap();
        np.add_incoming(&[(&n2, b2), (&n3, b3do), (&n4, b4do), (&n5, b4do5)]);
        let npi = np.as_basic_value().into_pointer_value();
        self.b.build_store(npi, self.i8t().const_zero()).unwrap();
        let len = self.b.build_int_sub(
            self.b.build_ptr_to_int(npi, self.i64t(), "ni").unwrap(),
            self.b.build_ptr_to_int(r, self.i64t(), "ri").unwrap(),
            "len",
        ).unwrap();
        let st = self.串型();
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, r, 0, "q0").unwrap().into_struct_value();
        sv = self.b.build_insert_value(sv, len, 1, "q1").unwrap().into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn str_eq_fn(&self) {
        let f = self.函数("prion_str_eq", Some(self.i64t().into()), &[self.p(), self.p()], false);
        if !self.入口(f) {
            return;
        }
        let a = f.get_nth_param(0).unwrap().into_pointer_value();
        let b = f.get_nth_param(1).unwrap().into_pointer_value();
        let st = self.串型();
        let one = self.i64t().const_int(1, false);
        let zero = self.i64t().const_zero();
        let b_a1 = self.块(f, "eqa1");
        let b_an = self.块(f, "eqan");
        let b_anb = self.块(f, "eqanb");
        let b_a1b = self.块(f, "eqa1b");
        let b_ap = self.块(f, "eqap");
        let b_bp = self.块(f, "eqbp");
        let b_cmp = self.块(f, "eqcmp");
        let b_mem = self.块(f, "eqmem");
        let b_r1 = self.块(f, "eqr1");
        let b_r0 = self.块(f, "eqr0");
        self.b
            .build_conditional_branch(self.b.build_is_null(a, "an").unwrap(), b_an, b_a1)
            .unwrap();

        self.b.position_at_end(b_a1);
        self.b
            .build_conditional_branch(self.b.build_is_null(b, "bn").unwrap(), b_r0, b_ap)
            .unwrap();

        self.b.position_at_end(b_an);
        self.b
            .build_conditional_branch(self.b.build_is_null(b, "bn2").unwrap(), b_r1, b_anb)
            .unwrap();

        self.b.position_at_end(b_anb);
        let bp2 = self.b.build_struct_gep(st, b, 0, "bp2").unwrap();
        let bv2 = self.b.build_load(self.p(), bp2, "bv2").unwrap().into_pointer_value();
        self.b
            .build_conditional_branch(self.b.build_is_null(bv2, "bn3").unwrap(), b_r1, b_r0)
            .unwrap();

        self.b.position_at_end(b_ap);
        let ap = self.b.build_struct_gep(st, a, 0, "ap").unwrap();
        let av = self.b.build_load(self.p(), ap, "av").unwrap().into_pointer_value();
        self.b
            .build_conditional_branch(self.b.build_is_null(av, "avn").unwrap(), b_a1b, b_bp)
            .unwrap();

        self.b.position_at_end(b_a1b);
        let bp3 = self.b.build_struct_gep(st, b, 0, "bp3").unwrap();
        let bv3 = self.b.build_load(self.p(), bp3, "bv3").unwrap().into_pointer_value();
        self.b
            .build_conditional_branch(self.b.build_is_null(bv3, "bn4").unwrap(), b_r1, b_r0)
            .unwrap();

        self.b.position_at_end(b_bp);
        let bpv = self.b.build_struct_gep(st, b, 0, "bpv").unwrap();
        let bvp = self.b.build_load(self.p(), bpv, "bvp").unwrap().into_pointer_value();
        self.b
            .build_conditional_branch(self.b.build_is_null(bvp, "bn5").unwrap(), b_r0, b_cmp)
            .unwrap();

        self.b.position_at_end(b_cmp);
        let anl = self.b.build_struct_gep(st, a, 1, "anl").unwrap();
        let anv = self.b.build_load(self.i64t(), anl, "anv").unwrap().into_int_value();
        let bnl = self.b.build_struct_gep(st, b, 1, "bnl").unwrap();
        let bnv = self.b.build_load(self.i64t(), bnl, "bnv").unwrap().into_int_value();
        self.b
            .build_conditional_branch(
                self.b.build_int_compare(IntPredicate::NE, anv, bnv, "ne").unwrap(),
                b_r0,
                b_mem,
            )
            .unwrap();

        self.b.position_at_end(b_mem);
        let m = self
            .调(self.memcmp(), &[av.into(), bvp.into(), anv.into()], "m")
            .unwrap()
            .into_int_value();
        let eq = self
            .b
            .build_int_compare(IntPredicate::EQ, m, self.i32t().const_zero(), "eqz")
            .unwrap();
        let r = self.b.build_select(eq, one, zero, "eqr").unwrap().into_int_value();
        self.b.build_return(Some(&r)).unwrap();

        self.b.position_at_end(b_r1);
        self.b.build_return(Some(&one)).unwrap();
        self.b.position_at_end(b_r0);
        self.b.build_return(Some(&zero)).unwrap();
    }

    fn str_index_fn(&self) {
        let f = self.函数("prion_str_index", Some(self.i64t().into()), &[self.p(), self.p()], false);
        if !self.入口(f) {
            return;
        }
        let s = f.get_nth_param(0).unwrap().into_pointer_value();
        let sub = f.get_nth_param(1).unwrap().into_pointer_value();
        let st = self.串型();
        let zero = self.i64t().const_zero();
        let one = self.i64t().const_int(1, false);
        let b_2 = self.块(f, "ix2");
        let b_3 = self.块(f, "ix3");
        let b_4 = self.块(f, "ix4");
        let b_sub = self.块(f, "ixsub");
        let b_5 = self.块(f, "ix5");
        let b_loop = self.块(f, "ixl");
        let b_body = self.块(f, "ixb");
        let b_r0 = self.块(f, "ixr0");
        let b_r1 = self.块(f, "ixr1");
        let b_found = self.块(f, "ixf");
        self.b
            .build_conditional_branch(self.b.build_is_null(s, "sn").unwrap(), b_r0, b_2)
            .unwrap();
        self.b.position_at_end(b_2);
        self.b
            .build_conditional_branch(self.b.build_is_null(sub, "n2").unwrap(), b_r0, b_3)
            .unwrap();
        self.b.position_at_end(b_3);
        let sp = self.b.build_struct_gep(st, s, 0, "sp").unwrap();
        let spv = self.b.build_load(self.p(), sp, "spv").unwrap().into_pointer_value();
        self.b
            .build_conditional_branch(self.b.build_is_null(spv, "n3").unwrap(), b_r0, b_4)
            .unwrap();
        self.b.position_at_end(b_4);
        let up = self.b.build_struct_gep(st, sub, 0, "up").unwrap();
        let upv = self.b.build_load(self.p(), up, "upv").unwrap().into_pointer_value();
        self.b
            .build_conditional_branch(self.b.build_is_null(upv, "n4").unwrap(), b_r0, b_sub)
            .unwrap();
        self.b.position_at_end(b_sub);
        let ul = self.b.build_struct_gep(st, sub, 1, "ul").unwrap();
        let uv = self.b.build_load(self.i64t(), ul, "uv").unwrap().into_int_value();
        self.b
            .build_conditional_branch(
                self.b.build_int_compare(IntPredicate::SLE, uv, zero, "le").unwrap(),
                b_r1,
                b_5,
            )
            .unwrap();
        self.b.position_at_end(b_5);
        let sl = self.b.build_struct_gep(st, s, 1, "sl").unwrap();
        let sv = self.b.build_load(self.i64t(), sl, "sv").unwrap().into_int_value();
        self.b.build_unconditional_branch(b_loop).unwrap();
        self.b.position_at_end(b_loop);
        let i = self.b.build_phi(self.i64t(), "ii").unwrap();
        i.add_incoming(&[(&zero, b_5)]);
        let ii = i.as_basic_value().into_int_value();
        let room = self.b.build_int_add(ii, uv, "rm").unwrap();
        let fits = self.b.build_int_compare(IntPredicate::SLE, room, sv, "fit").unwrap();
        self.b.build_conditional_branch(fits, b_body, b_r0).unwrap();
        self.b.position_at_end(b_body);
        let src = unsafe { self.b.build_gep(self.i8t(), spv, &[ii], "src") }.unwrap();
        let m = self
            .调(self.memcmp(), &[src.into(), upv.into(), uv.into()], "m")
            .unwrap()
            .into_int_value();
        let hit = self.b
            .build_int_compare(IntPredicate::EQ, m, self.i32t().const_zero(), "hit")
            .unwrap();
        let inn = self.b.build_int_add(ii, one, "inn").unwrap();
        self.b.build_conditional_branch(hit, b_found, b_loop).unwrap();
        i.add_incoming(&[(&inn, b_body)]);
        self.b.position_at_end(b_found);
        let pos = self.b.build_int_add(ii, one, "pos").unwrap();
        self.b.build_return(Some(&pos)).unwrap();
        self.b.position_at_end(b_r0);
        self.b.build_return(Some(&zero)).unwrap();
        self.b.position_at_end(b_r1);
        self.b.build_return(Some(&one)).unwrap();
    }

    fn int_of_str_fn(&self) {
        let f = self.函数("prion_int_of_str", Some(self.i64t().into()), &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let sp = f.get_nth_param(0).unwrap().into_pointer_value();
        let atoll = self.函数("atoll", Some(self.i64t().into()), &[self.p()], false);
        let (s, _) = self.取串(f, sp);
        let r = self.调(atoll, &[s.into()], "r").unwrap().into_int_value();
        self.b.build_return(Some(&r)).unwrap();
    }

    fn float_of_str_fn(&self) {
        let f = self.函数("prion_float_of_str", Some(self.ctx.f64_type().into()), &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let sp = f.get_nth_param(0).unwrap().into_pointer_value();
        let strtod = self.函数("strtod", Some(self.ctx.f64_type().into()), &[self.p(), self.p()], false);
        let (s, _) = self.取串(f, sp);
        let z = self.ctx.f64_type().const_zero();
        let p = self.pt().const_null();
        let r = self.调(strtod, &[s.into(), p.into()], "r").unwrap();
        let _ = z;
        self.b.build_return(Some(&r)).unwrap();
    }

    fn str_of_int_fn(&self) {
        let f = self.函数("prion_str_of_int", None, &[self.p(), self.i64t().into()], false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let v = f.get_nth_param(1).unwrap().into_int_value();
        let tmp = self.b.build_alloca(self.i8t().array_type(21), "tmp").unwrap();
        let zero = self.i64t().const_zero();
        let ten = self.i64t().const_int(10, false);
        let negv = self.b.build_int_compare(IntPredicate::SLT, v, zero, "neg").unwrap();
        let nv = self.b.build_int_sub(zero, v, "nv").unwrap();
        let u0 = self.b.build_select(negv, nv, v, "u0").unwrap().into_int_value();
        let b_loop = self.块(f, "iol");
        let b_body = self.块(f, "iob");
        let b_done = self.块(f, "iod");
        self.b.build_unconditional_branch(b_loop).unwrap();
        self.b.position_at_end(b_loop);
        let n = self.b.build_phi(self.i64t(), "in").unwrap();
        n.add_incoming(&[(&zero, self.入口块(f))]);
        let u = self.b.build_phi(self.i64t(), "iu").unwrap();
        u.add_incoming(&[(&u0, self.入口块(f))]);
        let ni = n.as_basic_value().into_int_value();
        let ui = u.as_basic_value().into_int_value();
        self.b.build_unconditional_branch(b_body).unwrap();
        self.b.position_at_end(b_body);
        let d = self.b.build_int_unsigned_rem(ui, ten, "d").unwrap();
        let ch = self.b.build_int_add(d, self.i64t().const_int(0x30, false), "ch").unwrap();
        let ta = unsafe { self.b.build_gep(self.i8t(), tmp, &[ni], "ta") }.unwrap();
        self.b.build_store(ta, self.b.build_int_truncate(ch, self.i8t(), "chb").unwrap()).unwrap();
        let n1 = self.b.build_int_add(ni, self.i64t().const_int(1, false), "n1").unwrap();
        let u1 = self.b.build_int_unsigned_div(ui, ten, "u1").unwrap();
        let stop = self.b.build_int_compare(IntPredicate::EQ, u1, zero, "stop").unwrap();
        n.add_incoming(&[(&n1, b_body)]);
        u.add_incoming(&[(&u1, b_body)]);
        self.b.build_conditional_branch(stop, b_done, b_loop).unwrap();
        self.b.position_at_end(b_done);
        let neg8 = self.b.build_int_z_extend(negv, self.i64t(), "ng").unwrap();
        let total = self.b.build_int_add(n1, neg8, "tot").unwrap();
        let sz = self.b.build_int_add(total, self.i64t().const_int(1, false), "sz").unwrap();
        let alloc = self.函数("prion_str_alloc", Some(self.p()), &[self.i64t().into()], false);
        let b = self.调(alloc, &[sz.into()], "b").unwrap().into_pointer_value();
        let b_minus = self.块(f, "iom");
        let b_fill = self.块(f, "iof");
        self.b.build_conditional_branch(negv, b_minus, b_fill).unwrap();
        self.b.position_at_end(b_minus);
        self.b.build_store(b, self.i8t().const_int(0x2D, false)).unwrap();
        let bo = unsafe { self.b.build_gep(self.i8t(), b, &[self.i64t().const_int(1, false)], "bo") }.unwrap();
        self.b.build_unconditional_branch(b_fill).unwrap();
        self.b.position_at_end(b_fill);
        let k0 = self.b.build_phi(self.i64t(), "ik0").unwrap();
        k0.add_incoming(&[(&zero, b_done), (&self.i64t().const_int(1, false), b_minus)]);
        let bp = self.b.build_phi(self.p(), "ibp").unwrap();
        bp.add_incoming(&[(&b, b_done), (&bo, b_minus)]);
        let b_fl = self.块(f, "ifl");
        let b_fb = self.块(f, "ifb");
        let b_fd = self.块(f, "ifd");
        self.b.build_unconditional_branch(b_fl).unwrap();
        self.b.position_at_end(b_fl);
        let n2 = self.b.build_phi(self.i64t(), "in2").unwrap();
        n2.add_incoming(&[(&n1, b_fill)]);
        let k2 = self.b.build_phi(self.i64t(), "ik2").unwrap();
        k2.add_incoming(&[(&k0.as_basic_value().into_int_value(), b_fill)]);
        let bp2 = self.b.build_phi(self.p(), "ibp2").unwrap();
        bp2.add_incoming(&[(&bp.as_basic_value().into_pointer_value(), b_fill)]);
        let n2i = n2.as_basic_value().into_int_value();
        let k2i = k2.as_basic_value().into_int_value();
        let bp2i = bp2.as_basic_value().into_pointer_value();
        let more = self.b.build_int_compare(IntPredicate::SGT, n2i, zero, "more").unwrap();
        self.b.build_conditional_branch(more, b_fb, b_fd).unwrap();
        self.b.position_at_end(b_fb);
        let n3 = self.b.build_int_sub(n2i, self.i64t().const_int(1, false), "n3").unwrap();
        let ta2 = unsafe { self.b.build_gep(self.i8t(), tmp, &[n3], "ta2") }.unwrap();
        let tc = self.b.build_load(self.i8t(), ta2, "tc").unwrap().into_int_value();
        self.b.build_store(bp2i, tc).unwrap();
        let k3 = self.b.build_int_add(k2i, self.i64t().const_int(1, false), "k3").unwrap();
        let bp3 = unsafe { self.b.build_gep(self.i8t(), bp2i, &[self.i64t().const_int(1, false)], "bp3") }.unwrap();
        n2.add_incoming(&[(&n3, b_fb)]);
        k2.add_incoming(&[(&k3, b_fb)]);
        bp2.add_incoming(&[(&bp3, b_fb)]);
        self.b.build_unconditional_branch(b_fl).unwrap();
        self.b.position_at_end(b_fd);
        let endp = bp2.as_basic_value().into_pointer_value();
        self.b.build_store(endp, self.i8t().const_zero()).unwrap();
        let st = self.串型();
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, b, 0, "r0").unwrap().into_struct_value();
        sv = self
            .b
            .build_insert_value(sv, k2.as_basic_value().into_int_value(), 1, "r1")
            .unwrap()
            .into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn str_of_float_fn(&self) {
        let f = self.函数("prion_str_of_float", None, &[self.p(), self.ctx.f64_type().into()], false);
        if !self.入口(f) {
            return;
        }
        let sret = f.get_nth_param(0).unwrap().into_pointer_value();
        let d = f.get_nth_param(1).unwrap().into_float_value();
        let alloc = self.函数("prion_str_alloc", Some(self.p()), &[self.i64t().into()], false);
        let b = self.调(alloc, &[self.i64t().const_int(40, false).into()], "b")
            .unwrap()
            .into_pointer_value();
        let sn = self.函数(
            "snprintf",
            Some(self.i32t().into()),
            &[self.p(), self.i64t().into(), self.p()],
            true,
        );
        let fmt = self.串("%g");
        let n = self
            .调(sn, &[b.into(), self.i64t().const_int(40, false).into(), fmt.into(), d.into()], "n")
            .unwrap()
            .into_int_value();
        let pos = self.b
            .build_int_compare(IntPredicate::SGT, n, self.i32t().const_zero(), "pos")
            .unwrap();
        let n64 = self.b.build_int_s_extend(n, self.i64t(), "n64").unwrap();
        let len = self
            .b
            .build_select(pos, n64, self.i64t().const_zero(), "len")
            .unwrap()
            .into_int_value();
        let st = self.串型();
        let mut sv = st.get_undef();
        sv = self.b.build_insert_value(sv, b, 0, "f0").unwrap().into_struct_value();
        sv = self.b.build_insert_value(sv, len, 1, "f1").unwrap().into_struct_value();
        self.b.build_store(sret, sv).unwrap();
        self.b.build_return(None).unwrap();
    }

    fn 容器(&self, release: bool) -> Result<(), String> {
        let on = if release { "false" } else { "true" };
        let body = LIST_IR_BODY.replace("@@ON@@", on);
        let arith = ARITH_IR_BODY.replace("@@ON@@", on);
        let ir = format!("{}\n{}\n{}", ir_lits(), body, arith);
        self.并入_ir(&ir, "rt.list")
    }

    fn 并入_ir(&self, ir: &str, name: &str) -> Result<(), String> {
        let buf = inkwell::memory_buffer::MemoryBuffer::create_from_memory_range_copy(
            ir.as_bytes(),
            name,
        );
        let m = self.ctx.create_module_from_ir(buf).map_err(|e| e.to_string())?;
        self.m.link_in_module(m).map_err(|e| e.to_string())
    }

    fn 进程(&self) -> Result<(), String> {
        if self.windows {
            self.run();
            Ok(())
        } else {
            self.并入_ir(POSIX_RUN_IR, "rt.run")
        }
    }

    fn 系统(&self) {
        self.sleep();
        self.random();
        self.now();
        self.sqrt();
    }

    fn sqrt(&self) {
        let ft = BasicTypeEnum::from(self.ctx.f64_type());
        let f = self.函数("prion_sqrt", Some(ft), &[ft], false);
        if !self.入口(f) {
            return;
        }
        let x = f.get_nth_param(0).unwrap().into_float_value();
        let sf = self.函数("sqrt", Some(ft), &[ft], false);
        let r = self.调(sf, &[x.into()], "sq").unwrap();
        self.b.build_return(Some(&r)).unwrap();
    }

    fn print_raw(&self) {
        let f = self.函数("prion_print_raw", None, &[self.p()], false);
        if !self.入口(f) {
            return;
        }
        let sp = f.get_nth_param(0).unwrap().into_pointer_value();
        let st = self.串型();
        let b_null = self.块(f, "prn");
        let b_go = self.块(f, "prg");
        let isnull = self.b.build_is_null(sp, "spn").unwrap();
        self.b.build_conditional_branch(isnull, b_null, b_go).unwrap();
        self.b.position_at_end(b_null);
        self.b.build_return(None).unwrap();
        self.b.position_at_end(b_go);
        let pp = self.b.build_struct_gep(st, sp, 0, "prp").unwrap();
        let pv = self.b.build_load(self.p(), pp, "prv").unwrap().into_pointer_value();
        let lp = self.b.build_struct_gep(st, sp, 1, "prl").unwrap();
        let ln = self.b.build_load(self.i64t(), lp, "plen").unwrap();
        let out = if self.windows {
            let iob = self.函数("__acrt_iob_func", Some(self.p()), &[self.i32t().into()], false);
            self.调(iob, &[self.i32t().const_int(1, false).into()], "out")
                .unwrap()
                .into_pointer_value()
        } else {
            let g = self.外ptr("stdout");
            self.b.build_load(self.p(), g.as_pointer_value(), "out").unwrap().into_pointer_value()
        };
        let fw = self.函数(
            "fwrite",
            Some(self.i64t().into()),
            &[self.p(), self.i64t().into(), self.i64t().into(), self.p()],
            false,
        );
        self.调(fw, &[pv.into(), self.i64t().const_int(1, false).into(), ln.into(), out.into()], "");
        self.b.build_return(None).unwrap();
    }

    fn now(&self) {
        let f = self.函数("prion_now", Some(self.i64t().into()), &[], false);
        if !self.入口(f) {
            return;
        }
        let 十亿 = self.i64t().const_int(1_000_000_000, false);
        if self.windows {
            let fr = self.内部tli64("prion_qpfreq", 0);
            let bp = fr.as_pointer_value();
            let cur = self.b.build_load(self.i64t(), bp, "f0").unwrap().into_int_value();
            let 空 = self
                .b
                .build_int_compare(IntPredicate::EQ, cur, self.i64t().const_zero(), "fz")
                .unwrap();
            let b_init = self.块(f, "qfi");
            let b_get = self.块(f, "qfg");
            self.b.build_conditional_branch(空, b_init, b_get).unwrap();
            self.b.position_at_end(b_init);
            let tf = self.b.build_alloca(self.i64t(), "freq").unwrap();
            let qf = self.函数("QueryPerformanceFrequency", Some(self.i32t().into()), &[self.p()], false);
            self.调(qf, &[tf.into()], "");
            let fv = self.b.build_load(self.i64t(), tf, "fv").unwrap();
            self.b.build_store(bp, fv).unwrap();
            self.b.build_unconditional_branch(b_get).unwrap();
            self.b.position_at_end(b_get);
            let freq = self.b.build_load(self.i64t(), bp, "fq").unwrap().into_int_value();
            let tc = self.b.build_alloca(self.i64t(), "cnt").unwrap();
            let qc = self.函数("QueryPerformanceCounter", Some(self.i32t().into()), &[self.p()], false);
            self.调(qc, &[tc.into()], "");
            let c = self.b.build_load(self.i64t(), tc, "cv").unwrap().into_int_value();
            let q = self.b.build_int_signed_div(c, freq, "q").unwrap();
            let r = self.b.build_int_signed_rem(c, freq, "r").unwrap();
            let qn = self.b.build_int_mul(q, 十亿, "qn").unwrap();
            let rn = self.b.build_int_mul(r, 十亿, "rn").unwrap();
            let rq = self.b.build_int_signed_div(rn, freq, "rq").unwrap();
            let ns = self.b.build_int_add(qn, rq, "ns").unwrap();
            self.b.build_return(Some(&ns)).unwrap();
        } else {
            let ty = self.i64t().array_type(2);
            let ts = self.b.build_alloca(ty, "ts").unwrap();
            let cg = self.函数("clock_gettime", Some(self.i32t().into()), &[self.i32t().into(), self.p()], false);
            self.调(cg, &[self.i32t().const_int(1, false).into(), ts.into()], "");
            let z = self.i64t().const_zero();
            let sp = unsafe { self.b.build_gep(ty, ts, &[z, z], "sp") }.unwrap();
            let np = unsafe { self.b.build_gep(ty, ts, &[z, self.i64t().const_int(1, false)], "np") }.unwrap();
            let sec = self.b.build_load(self.i64t(), sp, "sec").unwrap().into_int_value();
            let nsec = self.b.build_load(self.i64t(), np, "nsec").unwrap().into_int_value();
            let sn = self.b.build_int_mul(sec, 十亿, "sn").unwrap();
            let ns = self.b.build_int_add(sn, nsec, "n2").unwrap();
            self.b.build_return(Some(&ns)).unwrap();
        }
    }

    fn 并发(&self) -> Result<(), String> {
        let mut ir = String::from(THREAD_SHARED_IR);
        ir.push_str(if self.windows { THREAD_WIN_IR } else { THREAD_POSIX_IR });
        self.并入_ir(&ir, "rt.thread")
    }

    fn sleep(&self) {
        let f = self.函数("prion_sleep", None, &[self.i64t().into()], false);
        if !self.入口(f) {
            return;
        }
        let ms = f.get_nth_param(0).unwrap().into_int_value();
        if self.windows {
            let sl = self.函数("Sleep", None, &[self.i32t().into()], false);
            let v = self.b.build_int_truncate(ms, self.i32t(), "ms32").unwrap();
            self.调(sl, &[v.into()], "");
        } else {
            let us = self.函数("usleep", Some(self.i32t().into()), &[self.i32t().into()], false);
            let k = self.b.build_int_mul(ms, self.i64t().const_int(1000, false), "us").unwrap();
            let v = self.b.build_int_truncate(k, self.i32t(), "us32").unwrap();
            self.调(us, &[v.into()], "");
        }
        self.b.build_return(None).unwrap();
    }

    fn random(&self) {
        let f = self.函数("prion_random", Some(self.i64t().into()), &[self.i64t().into()], false);
        if !self.入口(f) {
            return;
        }
        let n = f.get_nth_param(0).unwrap().into_int_value();
        let g = self.内部i32("prion_rand_seeded");
        let s = self.b.build_load(self.i32t(), g.as_pointer_value(), "sd").unwrap().into_int_value();
        let seeded = self
            .b
            .build_int_compare(IntPredicate::NE, s, self.i32t().const_zero(), "sd2")
            .unwrap();
        let b_seed = self.块(f, "sd");
        let b_go = self.块(f, "rnd");
        self.b.build_conditional_branch(seeded, b_go, b_seed).unwrap();
        self.b.position_at_end(b_seed);
        let tf = self.函数("time", Some(self.i64t().into()), &[self.p()], false);
        let tv = self.调(tf, &[self.pt().const_null().into()], "tv").unwrap().into_int_value();
        let t32 = self.b.build_int_truncate(tv, self.i32t(), "t32").unwrap();
        let srf = self.函数("srand", None, &[self.i32t().into()], false);
        self.调(srf, &[t32.into()], "");
        self.b.build_store(g.as_pointer_value(), self.i32t().const_int(1, false)).unwrap();
        self.b.build_unconditional_branch(b_go).unwrap();
        self.b.position_at_end(b_go);
        let le = self
            .b
            .build_int_compare(IntPredicate::SLE, n, self.i64t().const_zero(), "le")
            .unwrap();
        let nn = self
            .b
            .build_select(le, self.i64t().const_int(1, false), n, "nn")
            .unwrap()
            .into_int_value();
        let rf = self.函数("rand", Some(self.i32t().into()), &[], false);
        let r = self.调(rf, &[], "rv").unwrap().into_int_value();
        let r64 = self.b.build_int_z_extend(r, self.i64t(), "r64").unwrap();
        let m = self.b.build_int_unsigned_rem(r64, nn, "rm").unwrap();
        self.b.build_return(Some(&m)).unwrap();
    }

    fn 块(&self, f: FunctionValue<'ctx>, name: &str) -> inkwell::basic_block::BasicBlock<'ctx> {
        self.ctx.append_basic_block(f, name)
    }

    fn 宽串(&self, s: &str) -> PointerValue<'ctx> {
        let units: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
        let vals: Vec<IntValue<'ctx>> = units
            .iter()
            .map(|u| self.ctx.i16_type().const_int(*u as u64, false))
            .collect();
        let arr = self.ctx.i16_type().const_array(&vals);
        let g = self.m.add_global(arr.get_type(), None, "rt.w");
        g.set_initializer(&arr);
        g.set_constant(true);
        g.set_linkage(Linkage::Internal);
        g.as_pointer_value()
    }

    fn run(&self) {
        let ps = [self.p(), self.p(), self.p(), self.p()];
        let f = self.函数("prion_run", Some(self.i64t().into()), &ps, false);
        if !self.入口(f) {
            return;
        }
        let prog_in = f.get_nth_param(0).unwrap().into_pointer_value();
        let args = f.get_nth_param(1).unwrap().into_pointer_value();
        let out_in = f.get_nth_param(2).unwrap().into_pointer_value();
        let err_in = f.get_nth_param(3).unwrap().into_pointer_value();
        let st = self.串型();
        let i64z = self.i64t().const_zero();
        let neg = |v: i64| -> BasicValueEnum<'ctx> {
            self.i64t().const_int(v as u64, true).into()
        };

        let b_po = self.块(f, "po");
        let b_pn = self.块(f, "pn");
        let b_prog = self.块(f, "prog");
        self.b.build_conditional_branch(
            self.b.build_is_null(prog_in, "pin").unwrap(),
            b_pn,
            b_po,
        ).unwrap();
        self.b.position_at_end(b_po);
        let pp = self.b.build_struct_gep(st, prog_in, 0, "pp").unwrap();
        let pv = self.b.build_load(self.p(), pp, "pv").unwrap().into_pointer_value();
        let e1 = self.串("");
        let prog0 = self
            .b
            .build_select(self.b.build_is_null(pv, "pvn").unwrap(), e1, pv, "psel")
            .unwrap()
            .into_pointer_value();
        self.b.build_unconditional_branch(b_prog).unwrap();
        self.b.position_at_end(b_pn);
        let e2 = self.串("");
        self.b.build_unconditional_branch(b_prog).unwrap();
        self.b.position_at_end(b_prog);
        let prog = self.b.build_phi(self.p(), "progv").unwrap();
        prog.add_incoming(&[(&prog0, b_po), (&e2, b_pn)]);
        let prog = prog.as_basic_value().into_pointer_value();

        let c0 = self.b.build_load(self.i8t(), prog, "c0").unwrap().into_int_value();
        let empty = self.b.build_int_compare(IntPredicate::EQ, c0, self.i8t().const_zero(), "emp").unwrap();
        let b_go = self.块(f, "rgo");
        let b_404 = self.块(f, "r404");
        self.b.build_conditional_branch(empty, b_404, b_go).unwrap();
        self.b.position_at_end(b_404);
        self.b.build_return(Some(&neg(-404))).unwrap();

        self.b.position_at_end(b_go);
        let wcmd = self.b.build_alloca(self.ctx.i16_type().array_type(16384), "wcmd").unwrap();
        let wtmp = self.b.build_alloca(self.ctx.i16_type().array_type(4096), "wtmp").unwrap();
        let mb = self.函数(
            "MultiByteToWideChar",
            Some(self.i32t().into()),
            &[self.i32t().into(), self.i32t().into(), self.p(), self.i32t().into(), self.p(), self.i32t().into()],
            false,
        );
        let 转换 = |s: PointerValue<'ctx>| -> IntValue<'ctx> {
            self.调(
                mb,
                &[
                    self.i32t().const_int(65001, false).into(),
                    self.i32t().const_zero().into(),
                    s.into(),
                    self.i32t().const_int(u64::MAX, true).into(),
                    wtmp.into(),
                    self.i32t().const_int(4096, false).into(),
                ],
                "wc",
            )
            .unwrap()
            .into_int_value()
        };
        let b_cmd = self.块(f, "rcmd");
        let b_bad = self.块(f, "rbad");
        let n0 = 转换(prog);
        let okc = self.b.build_int_compare(IntPredicate::SGT, n0, self.i32t().const_zero(), "okc").unwrap();
        self.b.build_conditional_branch(okc, b_cmd, b_bad).unwrap();
        self.b.position_at_end(b_bad);
        self.b.build_return(Some(&neg(-10086))).unwrap();

        self.b.position_at_end(b_cmd);
        let wcpy = self.函数(
            "wcscpy_s",
            Some(self.i32t().into()),
            &[self.p(), self.i64t().into(), self.p()],
            false,
        );
        let wlen = self.函数("wcslen", Some(self.i64t().into()), &[self.p()], false);
        let q0 = unsafe { self.b.build_gep(self.ctx.i16_type(), wcmd, &[i64z], "q0") }.unwrap();
        self.b.build_store(q0, self.ctx.i16_type().const_int(0x22, false)).unwrap();
        let cmd1 = unsafe {
            self.b
                .build_gep(self.ctx.i16_type(), wcmd, &[self.i64t().const_int(1, false)], "c1")
        }
        .unwrap();
        self.调(
            wcpy,
            &[cmd1.into(), self.i64t().const_int(16383, false).into(), wtmp.into()],
            "",
        );
        let wl = self.调(wlen, &[wtmp.into()], "wl").unwrap().into_int_value();
        let lasti = self.b.build_int_add(wl, self.i64t().const_int(1, false), "li").unwrap();
        let last = unsafe { self.b.build_gep(self.ctx.i16_type(), wcmd, &[lasti], "lq") }.unwrap();
        self.b.build_store(last, self.ctx.i16_type().const_int(0x22, false)).unwrap();
        let endi = self.b.build_int_add(wl, self.i64t().const_int(2, false), "ei").unwrap();
        let endp = unsafe { self.b.build_gep(self.ctx.i16_type(), wcmd, &[endi], "en") }.unwrap();
        self.b.build_store(endp, self.ctx.i16_type().const_zero()).unwrap();
        let wcat = self.函数(
            "wcscat_s",
            Some(self.i32t().into()),
            &[self.p(), self.i64t().into(), self.p()],
            false,
        );
        let strchr = self.函数("strchr", Some(self.p()), &[self.p(), self.i32t().into()], false);

        let b_aloop = self.块(f, "aloop");
        let b_abody = self.块(f, "abody");
        let b_ac = self.块(f, "ac");
        let b_atest = self.块(f, "atest");
        let b_append = self.块(f, "aadd");
        let b_q1 = self.块(f, "aq1");
        let b_body = self.块(f, "ab2");
        let b_q2 = self.块(f, "aq2");
        let b_anext = self.块(f, "anext");
        let b_adone = self.块(f, "adone");

        let nullargs = self.b.build_is_null(args, "an").unwrap();
        self.b.build_conditional_branch(nullargs, b_adone, b_aloop).unwrap();

        self.b.position_at_end(b_aloop);
        let i = self.b.build_phi(self.i64t(), "ai").unwrap();
        i.add_incoming(&[(&i64z, b_cmd)]);
        let lenp = self.b.build_struct_gep(self.列型(), args, 0, "ln").unwrap();
        let len = self.b.build_load(self.i64t(), lenp, "len").unwrap().into_int_value();
        let ii = i.as_basic_value().into_int_value();
        let lt = self.b.build_int_compare(IntPredicate::SLT, ii, len, "lt").unwrap();
        self.b.build_conditional_branch(lt, b_abody, b_adone).unwrap();

        self.b.position_at_end(b_abody);
        let dp = self.b.build_struct_gep(self.列型(), args, 4, "dp").unwrap();
        let d = self.b.build_load(self.p(), dp, "d").unwrap().into_pointer_value();
        let ezp = self.b.build_struct_gep(self.列型(), args, 2, "ezp").unwrap();
        let esz = self.b.build_load(self.i64t(), ezp, "esz").unwrap().into_int_value();
        let off = self.b.build_int_mul(ii, esz, "off").unwrap();
        let slot = unsafe { self.b.build_gep(self.i8t(), d, &[off], "slot") }.unwrap();
        let avp = self.b.build_load(self.p(), slot, "av").unwrap().into_pointer_value();
        self.b.build_conditional_branch(
            self.b.build_is_null(avp, "avn").unwrap(),
            b_anext,
            b_ac,
        ).unwrap();

        self.b.position_at_end(b_ac);
        let a0 = self.b.build_load(self.i8t(), avp, "a0").unwrap().into_int_value();
        self.b.build_conditional_branch(
            self.b.build_int_compare(IntPredicate::EQ, a0, self.i8t().const_zero(), "ae").unwrap(),
            b_anext,
            b_atest,
        ).unwrap();

        self.b.position_at_end(b_atest);
        let n1 = 转换(avp);
        self.b.build_conditional_branch(
            self.b.build_int_compare(IntPredicate::SGT, n1, self.i32t().const_zero(), "ok1").unwrap(),
            b_append,
            b_anext,
        ).unwrap();

        self.b.position_at_end(b_append);
        let spc = self.宽串(" ");
        self.调(
            wcat,
            &[wcmd.into(), self.i64t().const_int(16384, false).into(), spc.into()],
            "",
        );
        let sp1 = self
            .调(strchr, &[avp.into(), self.i32t().const_int(32, false).into()], "s1")
            .unwrap()
            .into_pointer_value();
        self.b.build_conditional_branch(
            self.b.build_is_null(sp1, "s1n").unwrap(),
            b_body,
            b_q1,
        ).unwrap();

        self.b.position_at_end(b_q1);
        let qt = self.宽串("\"");
        self.调(
            wcat,
            &[wcmd.into(), self.i64t().const_int(16384, false).into(), qt.into()],
            "",
        );
        self.b.build_unconditional_branch(b_body).unwrap();

        self.b.position_at_end(b_body);
        self.调(
            wcat,
            &[wcmd.into(), self.i64t().const_int(16384, false).into(), wtmp.into()],
            "",
        );
        let sp2 = self
            .调(strchr, &[avp.into(), self.i32t().const_int(32, false).into()], "s2")
            .unwrap()
            .into_pointer_value();
        self.b.build_conditional_branch(
            self.b.build_is_null(sp2, "s2n").unwrap(),
            b_anext,
            b_q2,
        ).unwrap();

        self.b.position_at_end(b_q2);
        let qt2 = self.宽串("\"");
        self.调(
            wcat,
            &[wcmd.into(), self.i64t().const_int(16384, false).into(), qt2.into()],
            "",
        );
        self.b.build_unconditional_branch(b_anext).unwrap();

        self.b.position_at_end(b_anext);
        let inext = self.b.build_int_add(ii, self.i64t().const_int(1, false), "inx").unwrap();
        i.add_incoming(&[(&inext, b_anext)]);
        self.b.build_unconditional_branch(b_aloop).unwrap();

        self.b.position_at_end(b_adone);
        let ih = self
            .b
            .build_int_to_ptr(self.i64t().const_int(u64::MAX, true), self.pt(), "ih")
            .unwrap();
        let sa_ty = self.ctx.struct_type(
            &[
                self.i32t().into(),
                self.i8t().array_type(4).into(),
                self.pt().into(),
                self.i32t().into(),
                self.i8t().array_type(4).into(),
            ],
            false,
        );
        let sa = self.b.build_alloca(sa_ty, "sa").unwrap();
        self.b.build_store(sa, sa_ty.const_zero()).unwrap();
        let san = self.b.build_struct_gep(sa_ty, sa, 0, "san").unwrap();
        self.b.build_store(san, self.i32t().const_int(24, false)).unwrap();
        let sai = self.b.build_struct_gep(sa_ty, sa, 3, "sai").unwrap();
        self.b.build_store(sai, self.i32t().const_int(1, false)).unwrap();

        let si_ty = self.ctx.struct_type(
            &[
                self.i32t().into(),
                self.i8t().array_type(4).into(),
                self.pt().into(),
                self.pt().into(),
                self.pt().into(),
                self.i32t().into(),
                self.i32t().into(),
                self.i32t().into(),
                self.i32t().into(),
                self.i32t().into(),
                self.i32t().into(),
                self.i32t().into(),
                self.i32t().into(),
                self.ctx.i16_type().into(),
                self.ctx.i16_type().into(),
                self.i8t().array_type(4).into(),
                self.pt().into(),
                self.pt().into(),
                self.pt().into(),
                self.pt().into(),
            ],
            false,
        );
        let si = self.b.build_alloca(si_ty, "si").unwrap();
        self.b.build_store(si, si_ty.const_zero()).unwrap();
        let sicb = self.b.build_struct_gep(si_ty, si, 0, "sicb").unwrap();
        self.b.build_store(sicb, self.i32t().const_int(104, false)).unwrap();
        let pi_ty = self.ctx.struct_type(
            &[
                self.pt().into(),
                self.pt().into(),
                self.i32t().into(),
                self.i32t().into(),
            ],
            false,
        );
        let pi = self.b.build_alloca(pi_ty, "pi").unwrap();
        self.b.build_store(pi, pi_ty.const_zero()).unwrap();
        let ho = self.b.build_alloca(self.pt(), "ho").unwrap();
        self.b.build_store(ho, ih).unwrap();
        let he = self.b.build_alloca(self.pt(), "he").unwrap();
        self.b.build_store(he, ih).unwrap();
        let null = self.pt().const_null();

        let cfw = self.函数(
            "CreateFileW",
            Some(self.p()),
            &[
                self.p(),
                self.i32t().into(),
                self.i32t().into(),
                self.p(),
                self.i32t().into(),
                self.i32t().into(),
                self.p(),
            ],
            false,
        );
        let gsh = self.函数("GetStdHandle", Some(self.p()), &[self.i32t().into()], false);
        let cpw = self.函数(
            "CreateProcessW",
            Some(self.i32t().into()),
            &[
                self.p(),
                self.p(),
                self.p(),
                self.p(),
                self.i32t().into(),
                self.i32t().into(),
                self.p(),
                self.p(),
                self.p(),
                self.p(),
            ],
            false,
        );
        let wso = self.函数(
            "WaitForSingleObject",
            Some(self.i32t().into()),
            &[self.p(), self.i32t().into()],
            false,
        );
        let gec = self.函数(
            "GetExitCodeProcess",
            Some(self.i32t().into()),
            &[self.p(), self.p()],
            false,
        );
        let gle = self.函数("GetLastError", Some(self.i32t().into()), &[], false);

        let b_fail = self.块(f, "rf10086");
        let b_errchk = self.块(f, "errc");
        let b_outget = self.块(f, "outg");
        let b_outc = self.块(f, "outc");
        let b_outdo = self.块(f, "outd");
        let b_outopen = self.块(f, "outo");
        let b_errget = self.块(f, "errg");
        let b_errc = self.块(f, "errc2");
        let b_errdo = self.块(f, "errd");
        let b_erdie = self.块(f, "errdie");
        let b_erropen = self.块(f, "erro");
        let b_after = self.块(f, "after");
        let b_si = self.块(f, "rsi");
        let b_proc = self.块(f, "rproc");
        let b_pfail = self.块(f, "rpf");
        let b_wait = self.块(f, "rwait");

        self.b.build_conditional_branch(
            self.b.build_is_null(out_in, "on").unwrap(),
            b_errchk,
            b_outget,
        ).unwrap();

        self.b.position_at_end(b_outget);
        let opp = self.b.build_struct_gep(st, out_in, 0, "opp").unwrap();
        let opv = self.b.build_load(self.p(), opp, "opv").unwrap().into_pointer_value();
        self.b.build_conditional_branch(
            self.b.build_is_null(opv, "opn").unwrap(),
            b_errchk,
            b_outc,
        ).unwrap();

        self.b.position_at_end(b_outc);
        let o0 = self.b.build_load(self.i8t(), opv, "o0").unwrap().into_int_value();
        self.b.build_conditional_branch(
            self.b.build_int_compare(IntPredicate::EQ, o0, self.i8t().const_zero(), "oe").unwrap(),
            b_errchk,
            b_outdo,
        ).unwrap();

        self.b.position_at_end(b_outdo);
        let on = 转换(opv);
        self.b.build_conditional_branch(
            self.b.build_int_compare(IntPredicate::SGT, on, self.i32t().const_zero(), "onok").unwrap(),
            b_outopen,
            b_fail,
        ).unwrap();

        self.b.position_at_end(b_outopen);
        let h1 = self
            .调(
                cfw,
                &[
                    wtmp.into(),
                    self.i32t().const_int(0x40000000, false).into(),
                    self.i32t().const_zero().into(),
                    sa.into(),
                    self.i32t().const_int(2, false).into(),
                    self.i32t().const_int(0x80, false).into(),
                    null.into(),
                ],
                "h1",
            )
            .unwrap()
            .into_pointer_value();
        self.b.build_store(ho, h1).unwrap();
        let bad1 = self.b.build_int_compare(
            IntPredicate::EQ,
            self.b.build_ptr_to_int(h1, self.i64t(), "h1i").unwrap(),
            self.i64t().const_int(u64::MAX, true),
            "b1",
        ).unwrap();
        self.b.build_conditional_branch(bad1, b_fail, b_errchk).unwrap();

        self.b.position_at_end(b_errchk);
        self.b.build_conditional_branch(
            self.b.build_is_null(err_in, "en").unwrap(),
            b_after,
            b_errget,
        ).unwrap();

        self.b.position_at_end(b_errget);
        let epp = self.b.build_struct_gep(st, err_in, 0, "epp").unwrap();
        let epv = self.b.build_load(self.p(), epp, "epv").unwrap().into_pointer_value();
        self.b.build_conditional_branch(
            self.b.build_is_null(epv, "epn").unwrap(),
            b_after,
            b_errc,
        ).unwrap();

        self.b.position_at_end(b_errc);
        let e0 = self.b.build_load(self.i8t(), epv, "e0").unwrap().into_int_value();
        self.b.build_conditional_branch(
            self.b.build_int_compare(IntPredicate::EQ, e0, self.i8t().const_zero(), "ee").unwrap(),
            b_after,
            b_errdo,
        ).unwrap();

        self.b.position_at_end(b_errdo);
        let en = 转换(epv);
        self.b.build_conditional_branch(
            self.b.build_int_compare(IntPredicate::SGT, en, self.i32t().const_zero(), "enok").unwrap(),
            b_erropen,
            b_erdie,
        ).unwrap();

        self.b.position_at_end(b_erropen);
        let h2 = self
            .调(
                cfw,
                &[
                    wtmp.into(),
                    self.i32t().const_int(0x40000000, false).into(),
                    self.i32t().const_zero().into(),
                    sa.into(),
                    self.i32t().const_int(2, false).into(),
                    self.i32t().const_int(0x80, false).into(),
                    null.into(),
                ],
                "h2",
            )
            .unwrap()
            .into_pointer_value();
        self.b.build_store(he, h2).unwrap();
        let bad2 = self.b.build_int_compare(
            IntPredicate::EQ,
            self.b.build_ptr_to_int(h2, self.i64t(), "h2i").unwrap(),
            self.i64t().const_int(u64::MAX, true),
            "b2",
        ).unwrap();
        self.b.build_conditional_branch(bad2, b_erdie, b_after).unwrap();

        self.b.position_at_end(b_erdie);
        let hv = self.b.build_load(self.p(), ho, "hv").unwrap().into_pointer_value();
        self.关(f, hv);
        self.b.build_unconditional_branch(b_fail).unwrap();

        self.b.position_at_end(b_fail);
        self.b.build_return(Some(&neg(-10086))).unwrap();

        self.b.position_at_end(b_after);
        let hout = self.b.build_load(self.p(), ho, "hout").unwrap().into_pointer_value();
        let herr = self.b.build_load(self.p(), he, "herr").unwrap().into_pointer_value();
        let inv_o = self.b.build_int_compare(
            IntPredicate::EQ,
            self.b.build_ptr_to_int(hout, self.i64t(), "hoi").unwrap(),
            self.i64t().const_int(u64::MAX, true),
            "invo",
        ).unwrap();
        let inv_e = self.b.build_int_compare(
            IntPredicate::EQ,
            self.b.build_ptr_to_int(herr, self.i64t(), "hei").unwrap(),
            self.i64t().const_int(u64::MAX, true),
            "inve",
        ).unwrap();
        let both = self.b.build_and(inv_o, inv_e, "both").unwrap();
        self.b.build_conditional_branch(both, b_proc, b_si).unwrap();

        self.b.position_at_end(b_si);
        let fl = self.b.build_struct_gep(si_ty, si, 12, "fl").unwrap();
        let fv = self.b.build_load(self.i32t(), fl, "fv").unwrap().into_int_value();
        let nfl = self.b.build_or(fv, self.i32t().const_int(0x100, false), "nfl").unwrap();
        self.b.build_store(fl, nfl).unwrap();
        let g1 = self.调(gsh, &[self.i32t().const_int((-10i32) as u64, true).into()], "g1")
            .unwrap()
            .into_pointer_value();
        let p1 = self.b.build_struct_gep(si_ty, si, 17, "s1").unwrap();
        self.b.build_store(p1, g1).unwrap();
        let g2 = self.调(gsh, &[self.i32t().const_int((-11i32) as u64, true).into()], "g2")
            .unwrap()
            .into_pointer_value();
        let o2 = self.b.build_select(inv_o, g2, hout, "o2").unwrap().into_pointer_value();
        let p2 = self.b.build_struct_gep(si_ty, si, 18, "s2").unwrap();
        self.b.build_store(p2, o2).unwrap();
        let g3 = self.调(gsh, &[self.i32t().const_int((-12i32) as u64, true).into()], "g3")
            .unwrap()
            .into_pointer_value();
        let o3 = self.b.build_select(inv_e, g3, herr, "o3").unwrap().into_pointer_value();
        let p3 = self.b.build_struct_gep(si_ty, si, 19, "s3").unwrap();
        self.b.build_store(p3, o3).unwrap();
        self.b.build_unconditional_branch(b_proc).unwrap();

        self.b.position_at_end(b_proc);
        let ok = self
            .调(
                cpw,
                &[
                    null.into(),
                    wcmd.into(),
                    null.into(),
                    null.into(),
                    self.i32t().const_int(1, false).into(),
                    self.i32t().const_zero().into(),
                    null.into(),
                    null.into(),
                    si.into(),
                    pi.into(),
                ],
                "ok",
            )
            .unwrap()
            .into_int_value();
        let okz = self.b.build_int_compare(IntPredicate::NE, ok, self.i32t().const_zero(), "okz").unwrap();
        self.b.build_conditional_branch(okz, b_wait, b_pfail).unwrap();

        self.b.position_at_end(b_pfail);
        let err = self.调(gle, &[], "gle").unwrap().into_int_value();
        let hv2 = self.b.build_load(self.p(), ho, "hv2").unwrap().into_pointer_value();
        self.关(f, hv2);
        let he2 = self.b.build_load(self.p(), he, "he2").unwrap().into_pointer_value();
        self.关(f, he2);
        let r404 = self.块(f, "r404b");
        let r10086 = self.块(f, "r10086b");
        let e404 = self.b.build_int_compare(IntPredicate::EQ, err, self.i32t().const_int(2, false), "e2").unwrap();
        let e3 = self.b.build_int_compare(IntPredicate::EQ, err, self.i32t().const_int(3, false), "e3").unwrap();
        let is404 = self.b.build_or(e404, e3, "is404").unwrap();
        self.b.build_conditional_branch(is404, r404, r10086).unwrap();
        self.b.position_at_end(r404);
        self.b.build_return(Some(&neg(-404))).unwrap();
        self.b.position_at_end(r10086);
        self.b.build_return(Some(&neg(-10086))).unwrap();

        self.b.position_at_end(b_wait);
        let hproc = self.b.build_struct_gep(pi_ty, pi, 0, "hp").unwrap();
        let hpv = self.b.build_load(self.p(), hproc, "hpv").unwrap().into_pointer_value();
        let hthr = self.b.build_struct_gep(pi_ty, pi, 1, "ht").unwrap();
        let htv = self.b.build_load(self.p(), hthr, "htv").unwrap().into_pointer_value();
        self.调(wso, &[hpv.into(), self.i32t().const_int(u64::MAX, true).into()], "");
        let rcp = self.b.build_alloca(self.i32t(), "rc").unwrap();
        self.b.build_store(rcp, self.i32t().const_zero()).unwrap();
        self.调(gec, &[hpv.into(), rcp.into()], "");
        self.关(f, hpv);
        self.关(f, htv);
        let hout2 = self.b.build_load(self.p(), ho, "ho2").unwrap().into_pointer_value();
        self.关(f, hout2);
        let herr2 = self.b.build_load(self.p(), he, "he3").unwrap().into_pointer_value();
        self.关(f, herr2);
        let rc = self.b.build_load(self.i32t(), rcp, "rcv").unwrap().into_int_value();
        let big = self
            .b
            .build_int_compare(IntPredicate::UGE, rc, self.i32t().const_int(0xC0000000, true), "big")
            .unwrap();
        let b_crash = self.块(f, "rcrash");
        let b_ok = self.块(f, "rok");
        self.b.build_conditional_branch(big, b_crash, b_ok).unwrap();
        self.b.position_at_end(b_crash);
        self.b.build_return(Some(&neg(-4))).unwrap();
        self.b.position_at_end(b_ok);
        let rc64 = self.b.build_int_z_extend(rc, self.i64t(), "rc64").unwrap();
        self.b.build_return(Some(&rc64)).unwrap();
    }

    fn 关(&self, f: FunctionValue<'ctx>, h: PointerValue<'ctx>) {
        let close = self.函数("CloseHandle", Some(self.i32t().into()), &[self.p()], false);
        let b = self.块(f, "cl");
        let cont = self.块(f, "clc");
        let inv = self
            .b
            .build_int_compare(
                IntPredicate::EQ,
                self.b.build_ptr_to_int(h, self.i64t(), "hi").unwrap(),
                self.i64t().const_int(u64::MAX, true),
                "inv",
            )
            .unwrap();
        self.b.build_conditional_branch(inv, cont, b).unwrap();
        self.b.position_at_end(b);
        self.调(close, &[h.into()], "");
        self.b.build_unconditional_branch(cont).unwrap();
        self.b.position_at_end(cont);
    }
}


fn ir_lits() -> String {
    let mut s = String::new();
    for (n, v) in [
        ("oom1", "out of memory - the program requested more than the available memory"),
        ("oom2", "reduce the data size, or use a more memory-efficient approach"),
        ("pos", "index %lld out of bounds - this container has only %lld position(s)"),
        ("fix", "check the count with len(container) before accessing"),
        ("mkey", "key \"%s\" does not exist"),
        ("mkeyi", "key %lld does not exist"),
        ("mkeyfix", "check with has first; to add one use put"),
        ("ovf", "integer overflow - the add/sub/mul result exceeds the int range"),
        ("ovffix", "use float instead, or break the computation into smaller steps"),
        ("div0", "division by zero - the divisor cannot be 0"),
        ("div0fix", "check first with if (if divisor != 0:) before dividing"),
    ] {
        let bytes: Vec<u8> = v.as_bytes().iter().copied().chain(std::iter::once(0)).collect();
        let esc: String = bytes
            .iter()
            .map(|b| {
                if *b >= 0x20 && *b < 0x7f && *b != b'"' && *b != b'\\' {
                    (*b as char).to_string()
                } else {
                    format!("\\{:02X}", b)
                }
            })
            .collect();
        s.push_str(&format!(
            "@.s_{} = private unnamed_addr constant [{} x i8] c\"{}\"\n",
            n,
            bytes.len(),
            esc
        ));
    }
    s
}

const LIST_IR_BODY: &str = r##"
%rt.S = type { ptr, i64 }
%rt.L = type { i64, i64, i64, i64, ptr }

declare ptr @malloc(i64)
declare ptr @realloc(ptr, i64)
declare void @memcpy(ptr, ptr, i64)
declare void @memmove(ptr, ptr, i64)
declare i32 @printf(ptr, ...)
declare i32 @snprintf(ptr, i64, ptr, ...)
declare void @qsort(ptr, i64, i64, ptr)
declare void @prion_str_copy(ptr, ptr)
declare void @prion_str_empty(ptr)
declare void @prion_die_nc(i32, ptr, ptr)
declare void @prion_die_at(ptr, i64, i64, ptr, i32, ptr, ptr)

define i64 @prion_slot_get(ptr %p, i64 %esz, i64 %sgn) {
entry:
  %is8 = icmp eq i64 %esz, 8
  br i1 %is8, label %direct, label %copy
direct:
  %v = load i64, ptr %p
  ret i64 %v
copy:
  %tmp = alloca i64
  store i64 0, ptr %tmp
  call void @memcpy(ptr %tmp, ptr %p, i64 %esz)
  %v2 = load i64, ptr %tmp
  %lt = icmp ult i64 %esz, 8
  %snz = icmp ne i64 %sgn, 0
  %do = and i1 %lt, %snz
  br i1 %do, label %sh, label %done
sh:
  %bits = shl i64 %esz, 3
  %shv = sub i64 64, %bits
  %u = shl i64 %v2, %shv
  %r = ashr i64 %u, %shv
  ret i64 %r
done:
  ret i64 %v2
}

define void @prion_slot_put(ptr %p, i64 %esz, i64 %v) {
entry:
  %is8 = icmp eq i64 %esz, 8
  br i1 %is8, label %direct, label %copy
direct:
  store i64 %v, ptr %p
  ret void
copy:
  %tv = alloca i64
  store i64 %v, ptr %tv
  call void @memcpy(ptr %p, ptr %tv, i64 %esz)
  ret void
}

define ptr @prion_list_new(i64 %esz, i64 %sgn) {
entry:
  %l = call ptr @malloc(i64 40)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  store i64 0, ptr %f0
  %f1 = getelementptr %rt.L, ptr %l, i32 0, i32 1
  store i64 4, ptr %f1
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  store i64 %esz, ptr %f2
  %f3 = getelementptr %rt.L, ptr %l, i32 0, i32 3
  store i64 %sgn, ptr %f3
  %nb = mul i64 %esz, 4
  %d = call ptr @malloc(i64 %nb)
  %f4 = getelementptr %rt.L, ptr %l, i32 0, i32 4
  store ptr %d, ptr %f4
  %ln = icmp eq ptr %l, null
  %dn = icmp eq ptr %d, null
  %bad = or i1 %ln, %dn
  br i1 %bad, label %oom, label %ok
oom:
  call void @prion_die_nc(i32 6, ptr @.s_oom1, ptr @.s_oom2)
  ret ptr null
ok:
  ret ptr %l
}

define ptr @prion_list_slot(ptr %l, i64 %i) {
entry:
  %f4 = getelementptr %rt.L, ptr %l, i32 0, i32 4
  %d = load ptr, ptr %f4
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %o = mul i64 %i, %esz
  %p = getelementptr i8, ptr %d, i64 %o
  ret ptr %p
}

define i64 @prion_list_len(ptr %l) {
entry:
  %ln = icmp eq ptr %l, null
  br i1 %ln, label %zero, label %go
zero:
  ret i64 0
go:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %n = load i64, ptr %f0
  ret i64 %n
}

define void @prion_list_grow(ptr %l) {
entry:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %f1 = getelementptr %rt.L, ptr %l, i32 0, i32 1
  %cap = load i64, ptr %f1
  %full = icmp sge i64 %len, %cap
  br i1 %full, label %g, label %end
g:
  %ncap = mul i64 %cap, 2
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %sz = mul i64 %esz, %ncap
  %f4 = getelementptr %rt.L, ptr %l, i32 0, i32 4
  %d = load ptr, ptr %f4
  %nd = call ptr @realloc(ptr %d, i64 %sz)
  %nn = icmp eq ptr %nd, null
  br i1 %nn, label %oom, label %set
set:
  store ptr %nd, ptr %f4
  store i64 %ncap, ptr %f1
  br label %end
oom:
  call void @prion_die_nc(i32 6, ptr @.s_oom1, ptr @.s_oom2)
  br label %end
end:
  ret void
}

define void @prion_list_check(ptr %l, i64 %i, i64 %plus) {
entry:
  %ln = icmp eq ptr %l, null
  %on = and i1 @@ON@@, %ln
  br i1 %on, label %die0, label %chk
chk:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %lim = add i64 %len, %plus
  %lt = icmp slt i64 %i, 1
  %gt = icmp sgt i64 %i, %lim
  %bad = or i1 %lt, %gt
  %on2 = and i1 @@ON@@, %bad
  br i1 %on2, label %diev, label %ok
die0:
  call void @prion_list_die(i64 %i, i64 0)
  br label %ok
diev:
  call void @prion_list_die(i64 %i, i64 %lim)
  br label %ok
ok:
  ret void
}

define void @prion_list_check_at(ptr %l, i64 %i, i64 %plus, ptr %f, i64 %ln, i64 %cl) {
entry:
  %ln0 = icmp eq ptr %l, null
  %on = and i1 @@ON@@, %ln0
  br i1 %on, label %die0, label %chk
chk:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %lim = add i64 %len, %plus
  %lt = icmp slt i64 %i, 1
  %gt = icmp sgt i64 %i, %lim
  %bad = or i1 %lt, %gt
  %on2 = and i1 @@ON@@, %bad
  br i1 %on2, label %diev, label %ok
die0:
  call void @prion_list_die_at(i64 %i, i64 0, ptr %f, i64 %ln, i64 %cl)
  br label %ok
diev:
  call void @prion_list_die_at(i64 %i, i64 %lim, ptr %f, i64 %ln, i64 %cl)
  br label %ok
ok:
  ret void
}

define void @prion_list_check_pop(ptr %l) {
entry:
  %ln = icmp eq ptr %l, null
  %on = and i1 @@ON@@, %ln
  br i1 %on, label %die0, label %chk
chk:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %le = icmp sle i64 %len, 0
  %on2 = and i1 @@ON@@, %le
  br i1 %on2, label %diev, label %ok
die0:
  call void @prion_list_die(i64 0, i64 0)
  br label %ok
diev:
  call void @prion_list_die(i64 0, i64 %len)
  br label %ok
ok:
  ret void
}

define void @prion_list_die(i64 %i, i64 %n) {
entry:
  %b = alloca [192 x i8]
  %bp = getelementptr [192 x i8], ptr %b, i64 0, i64 0
  %c = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %bp, i64 192, ptr @.s_pos, i64 %i, i64 %n)
  call void @prion_die_nc(i32 4, ptr %bp, ptr @.s_fix)
  ret void
}

define void @prion_list_die_at(i64 %i, i64 %n, ptr %f, i64 %ln, i64 %cl) {
entry:
  %b = alloca [192 x i8]
  %bp = getelementptr [192 x i8], ptr %b, i64 0, i64 0
  %c = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %bp, i64 192, ptr @.s_pos, i64 %i, i64 %n)
  call void @prion_die_at(ptr %f, i64 %ln, i64 %cl, ptr null, i32 4, ptr %bp, ptr @.s_fix)
  ret void
}

define void @prion_list_reserve(ptr %l, i64 %n) {
entry:
  %ln = icmp eq ptr %l, null
  br i1 %ln, label %end, label %chk
chk:
  %f1 = getelementptr %rt.L, ptr %l, i32 0, i32 1
  %cap = load i64, ptr %f1
  %le = icmp sle i64 %n, %cap
  br i1 %le, label %end, label %go
go:
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %sz = mul i64 %esz, %n
  %f4 = getelementptr %rt.L, ptr %l, i32 0, i32 4
  %d = load ptr, ptr %f4
  %nd = call ptr @realloc(ptr %d, i64 %sz)
  %nn = icmp eq ptr %nd, null
  br i1 %nn, label %oom, label %set
set:
  store ptr %nd, ptr %f4
  store i64 %n, ptr %f1
  br label %end
oom:
  call void @prion_die_nc(i32 6, ptr @.s_oom1, ptr @.s_oom2)
  br label %end
end:
  ret void
}

define ptr @prion_list_dc_parent(ptr %h) {
entry:
  %hn = icmp eq ptr %h, null
  br i1 %hn, label %nul, label %go
nul:
  ret ptr null
go:
  %n = call ptr @malloc(i64 40)
  %h0 = getelementptr %rt.L, ptr %h, i32 0, i32 0
  %hl = load i64, ptr %h0
  %n0 = getelementptr %rt.L, ptr %n, i32 0, i32 0
  store i64 %hl, ptr %n0
  %h1 = getelementptr %rt.L, ptr %h, i32 0, i32 1
  %hc = load i64, ptr %h1
  %n1 = getelementptr %rt.L, ptr %n, i32 0, i32 1
  store i64 %hc, ptr %n1
  %h2 = getelementptr %rt.L, ptr %h, i32 0, i32 2
  %he = load i64, ptr %h2
  %n2 = getelementptr %rt.L, ptr %n, i32 0, i32 2
  store i64 %he, ptr %n2
  %h3 = getelementptr %rt.L, ptr %h, i32 0, i32 3
  %hs = load i64, ptr %h3
  %n3 = getelementptr %rt.L, ptr %n, i32 0, i32 3
  store i64 %hs, ptr %n3
  %cap0 = icmp sgt i64 %hc, 0
  %capv = select i1 %cap0, i64 %hc, i64 1
  %nb = mul i64 %he, %capv
  %d = call ptr @malloc(i64 %nb)
  %n4 = getelementptr %rt.L, ptr %n, i32 0, i32 4
  store ptr %d, ptr %n4
  %has = icmp sgt i64 %hl, 0
  br i1 %has, label %cp, label %done
cp:
  %h4 = getelementptr %rt.L, ptr %h, i32 0, i32 4
  %hd = load ptr, ptr %h4
  %sz = mul i64 %he, %hl
  call void @memcpy(ptr %d, ptr %hd, i64 %sz)
  br label %done
done:
  ret ptr %n
}

define void @prion_list_copy_at(ptr %l, i64 %i, ptr %dst) {
entry:
  call void @prion_list_check(ptr %l, i64 %i, i64 0)
  %im = sub i64 %i, 1
  %p = call ptr @prion_list_slot(ptr %l, i64 %im)
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  call void @memcpy(ptr %dst, ptr %p, i64 %esz)
  ret void
}

define void @prion_list_push_from(ptr %l, ptr %src) {
entry:
  call void @prion_list_grow(ptr %l)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %p = call ptr @prion_list_slot(ptr %l, i64 %len)
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  call void @memcpy(ptr %p, ptr %src, i64 %esz)
  %n = add i64 %len, 1
  store i64 %n, ptr %f0
  ret void
}

define void @prion_list_setat_from(ptr %l, i64 %i, ptr %src) {
entry:
  call void @prion_list_check(ptr %l, i64 %i, i64 0)
  %im = sub i64 %i, 1
  %p = call ptr @prion_list_slot(ptr %l, i64 %im)
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  call void @memcpy(ptr %p, ptr %src, i64 %esz)
  ret void
}

define void @prion_list_at_s(ptr %sret, ptr %l, i64 %i) {
entry:
  call void @prion_list_check(ptr %l, i64 %i, i64 0)
  %im = sub i64 %i, 1
  %p = call ptr @prion_list_slot(ptr %l, i64 %im)
  %v = load %rt.S, ptr %p
  store %rt.S %v, ptr %sret
  ret void
}

define void @prion_list_at_s_loc(ptr %sret, ptr %l, i64 %i, ptr %f, i64 %ln, i64 %cl, ptr %u) {
entry:
  call void @prion_list_check_at(ptr %l, i64 %i, i64 0, ptr %f, i64 %ln, i64 %cl)
  %im = sub i64 %i, 1
  %p = call ptr @prion_list_slot(ptr %l, i64 %im)
  %v = load %rt.S, ptr %p
  store %rt.S %v, ptr %sret
  ret void
}

define void @prion_list_setat_s(ptr %l, i64 %i, ptr %v) {
entry:
  call void @prion_list_check(ptr %l, i64 %i, i64 0)
  %im = sub i64 %i, 1
  %p = call ptr @prion_list_slot(ptr %l, i64 %im)
  %tmp = alloca %rt.S
  call void @prion_str_copy(ptr %tmp, ptr %v)
  %sv = load %rt.S, ptr %tmp
  store %rt.S %sv, ptr %p
  ret void
}

define i64 @prion_list_push_s(ptr %l, ptr %v) {
entry:
  call void @prion_list_grow(ptr %l)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %p = call ptr @prion_list_slot(ptr %l, i64 %len)
  %tmp = alloca %rt.S
  call void @prion_str_copy(ptr %tmp, ptr %v)
  %sv = load %rt.S, ptr %tmp
  store %rt.S %sv, ptr %p
  %n = add i64 %len, 1
  store i64 %n, ptr %f0
  ret i64 %n
}

define i64 @prion_list_insert_s(ptr %l, i64 %pos, ptr %v) {
entry:
  call void @prion_list_check(ptr %l, i64 %pos, i64 1)
  %z = alloca %rt.S
  store %rt.S zeroinitializer, ptr %z
  %r1 = call i64 @prion_list_push_s(ptr %l, ptr %z)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %i0 = sub i64 %len, 1
  br label %cond
cond:
  %i = phi i64 [ %i0, %entry ], [ %in, %body ]
  %ge = icmp sge i64 %i, %pos
  br i1 %ge, label %body, label %done
body:
  %im = sub i64 %i, 1
  %ps = call ptr @prion_list_slot(ptr %l, i64 %i)
  %pm = call ptr @prion_list_slot(ptr %l, i64 %im)
  call void @memcpy(ptr %ps, ptr %pm, i64 16)
  %in = sub i64 %i, 1
  br label %cond
done:
  %pp = sub i64 %pos, 1
  %dst = call ptr @prion_list_slot(ptr %l, i64 %pp)
  %tmp = alloca %rt.S
  call void @prion_str_copy(ptr %tmp, ptr %v)
  %sv = load %rt.S, ptr %tmp
  store %rt.S %sv, ptr %dst
  ret i64 %len
}

define void @prion_list_pop_s(ptr %sret, ptr %l) {
entry:
  call void @prion_list_check_pop(ptr %l)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %n = sub i64 %len, 1
  store i64 %n, ptr %f0
  %p = call ptr @prion_list_slot(ptr %l, i64 %n)
  %v = load %rt.S, ptr %p
  store %rt.S %v, ptr %sret
  ret void
}

define void @prion_list_remove_s(ptr %sret, ptr %l, i64 %pos) {
entry:
  call void @prion_list_check(ptr %l, i64 %pos, i64 0)
  %pp = sub i64 %pos, 1
  %src = call ptr @prion_list_slot(ptr %l, i64 %pp)
  %v = load %rt.S, ptr %src
  %dst = call ptr @prion_list_slot(ptr %l, i64 %pos)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %cnt = sub i64 %len, %pos
  %sz = mul i64 %cnt, 16
  call void @memmove(ptr %src, ptr %dst, i64 %sz)
  %n = sub i64 %len, 1
  store i64 %n, ptr %f0
  store %rt.S %v, ptr %sret
  ret void
}

define i64 @prion_list_at(ptr %l, i64 %i) {
entry:
  call void @prion_list_check(ptr %l, i64 %i, i64 0)
  %im = sub i64 %i, 1
  %p = call ptr @prion_list_slot(ptr %l, i64 %im)
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %f3 = getelementptr %rt.L, ptr %l, i32 0, i32 3
  %sgn = load i64, ptr %f3
  %v = call i64 @prion_slot_get(ptr %p, i64 %esz, i64 %sgn)
  ret i64 %v
}

define i64 @prion_list_at_loc(ptr %l, i64 %i, ptr %f, i64 %ln, i64 %cl, ptr %u) {
entry:
  call void @prion_list_check_at(ptr %l, i64 %i, i64 0, ptr %f, i64 %ln, i64 %cl)
  %im = sub i64 %i, 1
  %p = call ptr @prion_list_slot(ptr %l, i64 %im)
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %f3 = getelementptr %rt.L, ptr %l, i32 0, i32 3
  %sgn = load i64, ptr %f3
  %v = call i64 @prion_slot_get(ptr %p, i64 %esz, i64 %sgn)
  ret i64 %v
}

define void @prion_list_setat(ptr %l, i64 %i, i64 %v) {
entry:
  call void @prion_list_check(ptr %l, i64 %i, i64 0)
  %im = sub i64 %i, 1
  %p = call ptr @prion_list_slot(ptr %l, i64 %im)
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  call void @prion_slot_put(ptr %p, i64 %esz, i64 %v)
  ret void
}

define void @prion_list_setat_loc(ptr %l, i64 %i, i64 %v, ptr %f, i64 %ln, i64 %cl, ptr %u) {
entry:
  call void @prion_list_check_at(ptr %l, i64 %i, i64 0, ptr %f, i64 %ln, i64 %cl)
  %im = sub i64 %i, 1
  %p = call ptr @prion_list_slot(ptr %l, i64 %im)
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  call void @prion_slot_put(ptr %p, i64 %esz, i64 %v)
  ret void
}

define i64 @prion_list_push(ptr %l, i64 %v) {
entry:
  call void @prion_list_grow(ptr %l)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %p = call ptr @prion_list_slot(ptr %l, i64 %len)
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  call void @prion_slot_put(ptr %p, i64 %esz, i64 %v)
  %n = add i64 %len, 1
  store i64 %n, ptr %f0
  ret i64 %n
}

define i64 @prion_list_push_n(ptr %l, i64 %v, i64 %esz) {
entry:
  call void @prion_list_grow(ptr %l)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %f4 = getelementptr %rt.L, ptr %l, i32 0, i32 4
  %d = load ptr, ptr %f4
  %o = mul i64 %len, %esz
  %p = getelementptr i8, ptr %d, i64 %o
  call void @prion_slot_put(ptr %p, i64 %esz, i64 %v)
  %n = add i64 %len, 1
  store i64 %n, ptr %f0
  ret i64 %n
}

define i64 @prion_list_insert_from(ptr %l, i64 %pos, ptr %src) {
entry:
  call void @prion_list_check(ptr %l, i64 %pos, i64 1)
  %r0 = call i64 @prion_list_push(ptr %l, i64 0)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %i0 = sub i64 %len, 1
  br label %cond
cond:
  %i = phi i64 [ %i0, %entry ], [ %in, %body ]
  %ge = icmp sge i64 %i, %pos
  br i1 %ge, label %body, label %done
body:
  %im = sub i64 %i, 1
  %ps = call ptr @prion_list_slot(ptr %l, i64 %i)
  %pm = call ptr @prion_list_slot(ptr %l, i64 %im)
  call void @memmove(ptr %ps, ptr %pm, i64 %esz)
  %in = sub i64 %i, 1
  br label %cond
done:
  %pp = sub i64 %pos, 1
  %dst = call ptr @prion_list_slot(ptr %l, i64 %pp)
  call void @memcpy(ptr %dst, ptr %src, i64 %esz)
  ret i64 %len
}

define i64 @prion_list_insert(ptr %l, i64 %pos, i64 %v) {
entry:
  call void @prion_list_check(ptr %l, i64 %pos, i64 1)
  %r0 = call i64 @prion_list_push(ptr %l, i64 0)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %i0 = sub i64 %len, 1
  br label %cond
cond:
  %i = phi i64 [ %i0, %entry ], [ %in, %body ]
  %ge = icmp sge i64 %i, %pos
  br i1 %ge, label %body, label %done
body:
  %im = sub i64 %i, 1
  %ps = call ptr @prion_list_slot(ptr %l, i64 %i)
  %pm = call ptr @prion_list_slot(ptr %l, i64 %im)
  call void @memmove(ptr %ps, ptr %pm, i64 %esz)
  %in = sub i64 %i, 1
  br label %cond
done:
  %pp = sub i64 %pos, 1
  %dst = call ptr @prion_list_slot(ptr %l, i64 %pp)
  call void @prion_slot_put(ptr %dst, i64 %esz, i64 %v)
  ret i64 %len
}

define i64 @prion_list_remove(ptr %l, i64 %pos) {
entry:
  call void @prion_list_check(ptr %l, i64 %pos, i64 0)
  %pp = sub i64 %pos, 1
  %src = call ptr @prion_list_slot(ptr %l, i64 %pp)
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %f3 = getelementptr %rt.L, ptr %l, i32 0, i32 3
  %sgn = load i64, ptr %f3
  %v = call i64 @prion_slot_get(ptr %src, i64 %esz, i64 %sgn)
  %dst = call ptr @prion_list_slot(ptr %l, i64 %pos)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %cnt = sub i64 %len, %pos
  %sz = mul i64 %esz, %cnt
  call void @memmove(ptr %src, ptr %dst, i64 %sz)
  %n = sub i64 %len, 1
  store i64 %n, ptr %f0
  ret i64 %v
}

define i64 @prion_list_pop(ptr %l) {
entry:
  call void @prion_list_check_pop(ptr %l)
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %n = sub i64 %len, 1
  store i64 %n, ptr %f0
  %p = call ptr @prion_list_slot(ptr %l, i64 %n)
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %f3 = getelementptr %rt.L, ptr %l, i32 0, i32 3
  %sgn = load i64, ptr %f3
  %v = call i64 @prion_slot_get(ptr %p, i64 %esz, i64 %sgn)
  ret i64 %v
}

define void @prion_list_clear(ptr %l) {
entry:
  %ln = icmp eq ptr %l, null
  br i1 %ln, label %end, label %go
go:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  store i64 0, ptr %f0
  br label %end
end:
  ret void
}

define i64 @prion_list_contains(ptr %l, i64 %v) {
entry:
  %ln = icmp eq ptr %l, null
  br i1 %ln, label %zero, label %init
zero:
  ret i64 0
init:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %f3 = getelementptr %rt.L, ptr %l, i32 0, i32 3
  %sgn = load i64, ptr %f3
  br label %loop
loop:
  %i = phi i64 [ 0, %init ], [ %in, %next ]
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %zero
body:
  %p = call ptr @prion_list_slot(ptr %l, i64 %i)
  %x = call i64 @prion_slot_get(ptr %p, i64 %esz, i64 %sgn)
  %eq = icmp eq i64 %x, %v
  br i1 %eq, label %hit, label %next
hit:
  ret i64 1
next:
  %in = add i64 %i, 1
  br label %loop
}

define i64 @prion_list_index(ptr %l, i64 %v) {
entry:
  %ln = icmp eq ptr %l, null
  br i1 %ln, label %zero, label %init
zero:
  ret i64 0
init:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %f3 = getelementptr %rt.L, ptr %l, i32 0, i32 3
  %sgn = load i64, ptr %f3
  br label %loop
loop:
  %i = phi i64 [ 0, %init ], [ %in, %next ]
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %zero
body:
  %p = call ptr @prion_list_slot(ptr %l, i64 %i)
  %x = call i64 @prion_slot_get(ptr %p, i64 %esz, i64 %sgn)
  %eq = icmp eq i64 %x, %v
  br i1 %eq, label %hit, label %next
hit:
  %pos = add i64 %i, 1
  ret i64 %pos
next:
  %in = add i64 %i, 1
  br label %loop
}

define ptr @prion_list_lit(i64 %n, ptr %v, i64 %esz, i64 %sgn) {
entry:
  %l = call ptr @prion_list_new(i64 %esz, i64 %sgn)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %body ]
  %lt = icmp slt i64 %i, %n
  br i1 %lt, label %body, label %done
body:
  %o = mul i64 %i, 8
  %p = getelementptr i8, ptr %v, i64 %o
  %x = load i64, ptr %p
  %r = call i64 @prion_list_push(ptr %l, i64 %x)
  %in = add i64 %i, 1
  br label %loop
done:
  ret ptr %l
}

define ptr @prion_list_lit_from(i64 %n, ptr %v, i64 %esz) {
entry:
  %l = call ptr @prion_list_new(i64 %esz, i64 0)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %body ]
  %lt = icmp slt i64 %i, %n
  br i1 %lt, label %body, label %done
body:
  %p = call ptr @prion_list_slot(ptr %l, i64 %i)
  %o = mul i64 %i, %esz
  %src = getelementptr i8, ptr %v, i64 %o
  call void @memcpy(ptr %p, ptr %src, i64 %esz)
  %in = add i64 %i, 1
  br label %loop
done:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  store i64 %n, ptr %f0
  ret ptr %l
}

define ptr @prion_list_lit_s(i64 %n, ptr %v) {
entry:
  %l = call ptr @prion_list_new(i64 16, i64 0)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %body ]
  %lt = icmp slt i64 %i, %n
  br i1 %lt, label %body, label %done
body:
  %o = mul i64 %i, 16
  %p = getelementptr i8, ptr %v, i64 %o
  %r = call i64 @prion_list_push_s(ptr %l, ptr %p)
  %in = add i64 %i, 1
  br label %loop
done:
  ret ptr %l
}

@.lp_open = private unnamed_addr constant [2 x i8] c"[\00"
@.lp_close = private unnamed_addr constant [2 x i8] c"]\00"
@.lp_sep = private unnamed_addr constant [3 x i8] c", \00"
@.lp_int = private unnamed_addr constant [5 x i8] c"%lld\00"
@.lp_g = private unnamed_addr constant [3 x i8] c"%g\00"
@.lp_tag = private unnamed_addr constant [4 x i8] c"%s[\00"
@.lp_str = private unnamed_addr constant [5 x i8] c"\22%s\22\00"
@.empty = private unnamed_addr constant [1 x i8] zeroinitializer
@prion_sort_esz = internal global i64 8
@prion_sort_sgn = internal global i64 1

declare i32 @memcmp(ptr, ptr, i64)

define void @prion_list_print(ptr %l) {
entry:
  %r0 = call i32 (ptr, ...) @printf(ptr @.lp_open)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %next ]
  %len = call i64 @prion_list_len(ptr %l)
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %gt = icmp sgt i64 %i, 0
  br i1 %gt, label %sep, label %val
sep:
  %r1 = call i32 (ptr, ...) @printf(ptr @.lp_sep)
  br label %val
val:
  %ip = add i64 %i, 1
  %x = call i64 @prion_list_at(ptr %l, i64 %ip)
  %r2 = call i32 (ptr, ...) @printf(ptr @.lp_int, i64 %x)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  %r3 = call i32 (ptr, ...) @printf(ptr @.lp_close)
  ret void
}

define void @prion_list_print_tag(ptr %l, ptr %tag) {
entry:
  %r0 = call i32 (ptr, ...) @printf(ptr @.lp_tag, ptr %tag)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %next ]
  %len = call i64 @prion_list_len(ptr %l)
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %gt = icmp sgt i64 %i, 0
  br i1 %gt, label %sep, label %val
sep:
  %r1 = call i32 (ptr, ...) @printf(ptr @.lp_sep)
  br label %val
val:
  %ip = add i64 %i, 1
  %x = call i64 @prion_list_at(ptr %l, i64 %ip)
  %r2 = call i32 (ptr, ...) @printf(ptr @.lp_int, i64 %x)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  %r3 = call i32 (ptr, ...) @printf(ptr @.lp_close)
  ret void
}

define void @prion_list_print_f(ptr %l) {
entry:
  %r0 = call i32 (ptr, ...) @printf(ptr @.lp_open)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %next ]
  %len = call i64 @prion_list_len(ptr %l)
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %gt = icmp sgt i64 %i, 0
  br i1 %gt, label %sep, label %val
sep:
  %r1 = call i32 (ptr, ...) @printf(ptr @.lp_sep)
  br label %val
val:
  %ip = add i64 %i, 1
  %x = call i64 @prion_list_at(ptr %l, i64 %ip)
  %d = bitcast i64 %x to double
  %r2 = call i32 (ptr, ...) @printf(ptr @.lp_g, double %d)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  %r3 = call i32 (ptr, ...) @printf(ptr @.lp_close)
  ret void
}

define void @prion_list_print_tag_f(ptr %l, ptr %tag) {
entry:
  %r0 = call i32 (ptr, ...) @printf(ptr @.lp_tag, ptr %tag)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %next ]
  %len = call i64 @prion_list_len(ptr %l)
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %gt = icmp sgt i64 %i, 0
  br i1 %gt, label %sep, label %val
sep:
  %r1 = call i32 (ptr, ...) @printf(ptr @.lp_sep)
  br label %val
val:
  %ip = add i64 %i, 1
  %x = call i64 @prion_list_at(ptr %l, i64 %ip)
  %d = bitcast i64 %x to double
  %r2 = call i32 (ptr, ...) @printf(ptr @.lp_g, double %d)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  %r3 = call i32 (ptr, ...) @printf(ptr @.lp_close)
  ret void
}

define void @prion_list_print_s(ptr %l) {
entry:
  %r0 = call i32 (ptr, ...) @printf(ptr @.lp_open)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %next ]
  %len = call i64 @prion_list_len(ptr %l)
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %gt = icmp sgt i64 %i, 0
  br i1 %gt, label %sep, label %val
sep:
  %r1 = call i32 (ptr, ...) @printf(ptr @.lp_sep)
  br label %val
val:
  %p = call ptr @prion_list_slot(ptr %l, i64 %i)
  %vp = getelementptr %rt.S, ptr %p, i32 0, i32 0
  %v = load ptr, ptr %vp
  %isn = icmp eq ptr %v, null
  %sel = select i1 %isn, ptr @.empty, ptr %v
  %r2 = call i32 (ptr, ...) @printf(ptr @.lp_str, ptr %sel)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  %r3 = call i32 (ptr, ...) @printf(ptr @.lp_close)
  ret void
}

define void @prion_list_print_tag_s(ptr %l, ptr %tag) {
entry:
  %r0 = call i32 (ptr, ...) @printf(ptr @.lp_tag, ptr %tag)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %next ]
  %len = call i64 @prion_list_len(ptr %l)
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %gt = icmp sgt i64 %i, 0
  br i1 %gt, label %sep, label %val
sep:
  %r1 = call i32 (ptr, ...) @printf(ptr @.lp_sep)
  br label %val
val:
  %p = call ptr @prion_list_slot(ptr %l, i64 %i)
  %vp = getelementptr %rt.S, ptr %p, i32 0, i32 0
  %v = load ptr, ptr %vp
  %isn = icmp eq ptr %v, null
  %sel = select i1 %isn, ptr @.empty, ptr %v
  %r2 = call i32 (ptr, ...) @printf(ptr @.lp_str, ptr %sel)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  %r3 = call i32 (ptr, ...) @printf(ptr @.lp_close)
  ret void
}

define internal i32 @prion_cmp_slot(ptr %a, ptr %b) {
entry:
  %esz = load i64, ptr @prion_sort_esz
  %sgn = load i64, ptr @prion_sort_sgn
  %x = call i64 @prion_slot_get(ptr %a, i64 %esz, i64 %sgn)
  %y = call i64 @prion_slot_get(ptr %b, i64 %esz, i64 %sgn)
  %snz = icmp ne i64 %sgn, 0
  br i1 %snz, label %sg, label %ug
sg:
  %gt = icmp sgt i64 %x, %y
  %lt = icmp slt i64 %x, %y
  %g1 = select i1 %gt, i32 1, i32 0
  %g2 = select i1 %lt, i32 -1, i32 %g1
  ret i32 %g2
ug:
  %ugt = icmp ugt i64 %x, %y
  %ult = icmp ult i64 %x, %y
  %u1 = select i1 %ugt, i32 1, i32 0
  %u2 = select i1 %ult, i32 -1, i32 %u1
  ret i32 %u2
}

define void @prion_list_sort(ptr %l) {
entry:
  %fe = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %fe
  store i64 %esz, ptr @prion_sort_esz
  %fs = getelementptr %rt.L, ptr %l, i32 0, i32 3
  %sgn = load i64, ptr %fs
  store i64 %sgn, ptr @prion_sort_sgn
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %f4 = getelementptr %rt.L, ptr %l, i32 0, i32 4
  %d = load ptr, ptr %f4
  call void @qsort(ptr %d, i64 %len, i64 %esz, ptr @prion_cmp_slot)
  ret void
}

define internal i32 @prion_cmp_slot_s(ptr %a, ptr %b) {
entry:
  %xa = getelementptr %rt.S, ptr %a, i32 0, i32 0
  %xp = load ptr, ptr %xa
  %xna = getelementptr %rt.S, ptr %a, i32 0, i32 1
  %xn = load i64, ptr %xna
  %yb = getelementptr %rt.S, ptr %b, i32 0, i32 0
  %yp = load ptr, ptr %yb
  %ynb = getelementptr %rt.S, ptr %b, i32 0, i32 1
  %yn = load i64, ptr %ynb
  %shorter = icmp slt i64 %xn, %yn
  %n = select i1 %shorter, i64 %xn, i64 %yn
  %isn1 = icmp eq ptr %xp, null
  %p1 = select i1 %isn1, ptr @.empty, ptr %xp
  %isn2 = icmp eq ptr %yp, null
  %p2 = select i1 %isn2, ptr @.empty, ptr %yp
  %c = call i32 @memcmp(ptr %p1, ptr %p2, i64 %n)
  %cnz = icmp ne i32 %c, 0
  br i1 %cnz, label %retc, label %len
retc:
  ret i32 %c
len:
  %gt = icmp sgt i64 %xn, %yn
  %lt = icmp slt i64 %xn, %yn
  %l1 = select i1 %gt, i32 1, i32 0
  %l2 = select i1 %lt, i32 -1, i32 %l1
  ret i32 %l2
}

define void @prion_list_sort_s(ptr %l) {
entry:
  %fe = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %fe
  store i64 %esz, ptr @prion_sort_esz
  %fs = getelementptr %rt.L, ptr %l, i32 0, i32 3
  %sgn = load i64, ptr %fs
  store i64 %sgn, ptr @prion_sort_sgn
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %f4 = getelementptr %rt.L, ptr %l, i32 0, i32 4
  %d = load ptr, ptr %f4
  call void @qsort(ptr %d, i64 %len, i64 %esz, ptr @prion_cmp_slot_s)
  ret void
}

define i64 @prion_list_contains_s(ptr %l, ptr %v) {
entry:
  %ln = icmp eq ptr %l, null
  br i1 %ln, label %ret0, label %init
init:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  br label %loop
loop:
  %i = phi i64 [ 0, %init ], [ %in, %next ]
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %ret0
body:
  %p = call ptr @prion_list_slot(ptr %l, i64 %i)
  %ep = getelementptr %rt.S, ptr %p, i32 0, i32 0
  %ev = load ptr, ptr %ep
  %enp = getelementptr %rt.S, ptr %p, i32 0, i32 1
  %en = load i64, ptr %enp
  %enull = icmp eq ptr %ev, null
  %vnull = icmp eq ptr %v, null
  %bad0 = or i1 %enull, %vnull
  br i1 %bad0, label %next, label %vchk
vchk:
  %vp = getelementptr %rt.S, ptr %v, i32 0, i32 0
  %vv = load ptr, ptr %vp
  %vnl = icmp eq ptr %vv, null
  br i1 %vnl, label %next, label %cmp
cmp:
  %vnpp = getelementptr %rt.S, ptr %v, i32 0, i32 1
  %vn = load i64, ptr %vnpp
  %eqn = icmp eq i64 %en, %vn
  br i1 %eqn, label %docmp, label %next
docmp:
  %c = call i32 @memcmp(ptr %ev, ptr %vv, i64 %en)
  %cz = icmp eq i32 %c, 0
  br i1 %cz, label %hit, label %next
hit:
  ret i64 1
next:
  %in = add i64 %i, 1
  br label %loop
ret0:
  ret i64 0
}

define i64 @prion_list_index_s(ptr %l, ptr %v) {
entry:
  %ln = icmp eq ptr %l, null
  br i1 %ln, label %ret0, label %init
init:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  br label %loop
loop:
  %i = phi i64 [ 0, %init ], [ %in, %next ]
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %ret0
body:
  %p = call ptr @prion_list_slot(ptr %l, i64 %i)
  %ep = getelementptr %rt.S, ptr %p, i32 0, i32 0
  %ev = load ptr, ptr %ep
  %enp = getelementptr %rt.S, ptr %p, i32 0, i32 1
  %en = load i64, ptr %enp
  %enull = icmp eq ptr %ev, null
  %vnull = icmp eq ptr %v, null
  %bad0 = or i1 %enull, %vnull
  br i1 %bad0, label %next, label %vchk
vchk:
  %vp = getelementptr %rt.S, ptr %v, i32 0, i32 0
  %vv = load ptr, ptr %vp
  %vnl = icmp eq ptr %vv, null
  br i1 %vnl, label %next, label %cmp
cmp:
  %vnpp = getelementptr %rt.S, ptr %v, i32 0, i32 1
  %vn = load i64, ptr %vnpp
  %eqn = icmp eq i64 %en, %vn
  br i1 %eqn, label %docmp, label %next
docmp:
  %c = call i32 @memcmp(ptr %ev, ptr %vv, i64 %en)
  %cz = icmp eq i32 %c, 0
  br i1 %cz, label %hit, label %next
hit:
  %pos = add i64 %i, 1
  ret i64 %pos
next:
  %in = add i64 %i, 1
  br label %loop
ret0:
  ret i64 0
}

define ptr @prion_dc_list(ptr %h, i64 %str_elem) {
entry:
  %hn = icmp eq ptr %h, null
  br i1 %hn, label %nul, label %go
nul:
  ret ptr null
go:
  %n = call ptr @malloc(i64 40)
  %nn = icmp eq ptr %n, null
  br i1 %nn, label %oom, label %alloc
alloc:
  %h0 = getelementptr %rt.L, ptr %h, i32 0, i32 0
  %hl = load i64, ptr %h0
  %n0 = getelementptr %rt.L, ptr %n, i32 0, i32 0
  store i64 %hl, ptr %n0
  %h1 = getelementptr %rt.L, ptr %h, i32 0, i32 1
  %hc = load i64, ptr %h1
  %n1 = getelementptr %rt.L, ptr %n, i32 0, i32 1
  store i64 %hc, ptr %n1
  %h2 = getelementptr %rt.L, ptr %h, i32 0, i32 2
  %he = load i64, ptr %h2
  %n2 = getelementptr %rt.L, ptr %n, i32 0, i32 2
  store i64 %he, ptr %n2
  %h3 = getelementptr %rt.L, ptr %h, i32 0, i32 3
  %hs = load i64, ptr %h3
  %n3 = getelementptr %rt.L, ptr %n, i32 0, i32 3
  store i64 %hs, ptr %n3
  %nb = mul i64 %he, %hc
  %d = call ptr @malloc(i64 %nb)
  %n4 = getelementptr %rt.L, ptr %n, i32 0, i32 4
  store ptr %d, ptr %n4
  %dn = icmp eq ptr %d, null
  br i1 %dn, label %oom, label %loop
oom:
  call void @prion_die_nc(i32 6, ptr @.s_oom1, ptr @.s_oom2)
  ret ptr null
loop:
  %i = phi i64 [ 0, %alloc ], [ %in, %lnext ]
  %lt = icmp slt i64 %i, %hl
  br i1 %lt, label %lbody, label %done
lbody:
  %hsp = call ptr @prion_list_slot(ptr %h, i64 %i)
  %nsp = call ptr @prion_list_slot(ptr %n, i64 %i)
  %isstr = icmp ne i64 %str_elem, 0
  br i1 %isstr, label %scp, label %rcp
scp:
  %tmp = alloca %rt.S
  call void @prion_str_copy(ptr %tmp, ptr %hsp)
  %sv = load %rt.S, ptr %tmp
  store %rt.S %sv, ptr %nsp
  br label %lnext
rcp:
  call void @memcpy(ptr %nsp, ptr %hsp, i64 %he)
  br label %lnext
lnext:
  %in = add i64 %i, 1
  br label %loop
done:
  ret ptr %n
}

define ptr @prion_list_slice(ptr %l, i64 %a, i64 %b) {
entry:
  %ln = icmp eq ptr %l, null
  %lt1 = icmp slt i64 %a, 1
  %lt2 = icmp slt i64 %b, %a
  %bad0 = or i1 %ln, %lt1
  %bad1 = or i1 %bad0, %lt2
  %on = and i1 @@ON@@, %bad1
  br i1 %on, label %die0, label %chk
chk:
  %f0 = getelementptr %rt.L, ptr %l, i32 0, i32 0
  %len = load i64, ptr %f0
  %gt = icmp sgt i64 %b, %len
  %on2 = and i1 @@ON@@, %gt
  br i1 %on2, label %diev, label %go
die0:
  call void @prion_list_die(i64 %b, i64 0)
  br label %go
diev:
  call void @prion_list_die(i64 %b, i64 %len)
  br label %go
go:
  %f2 = getelementptr %rt.L, ptr %l, i32 0, i32 2
  %esz = load i64, ptr %f2
  %f3 = getelementptr %rt.L, ptr %l, i32 0, i32 3
  %sgn = load i64, ptr %f3
  %r = call ptr @prion_list_new(i64 %esz, i64 %sgn)
  br label %loop
loop:
  %i = phi i64 [ %a, %go ], [ %in, %lbody ]
  %le = icmp sle i64 %i, %b
  br i1 %le, label %lbody, label %done
lbody:
  %im = sub i64 %i, 1
  %p = call ptr @prion_list_slot(ptr %l, i64 %im)
  call void @prion_list_push_from(ptr %r, ptr %p)
  %in = add i64 %i, 1
  br label %loop
done:
  ret ptr %r
}

%rt.M = type { i64, i64, i64, i64, i64, i64, ptr, ptr, ptr, i64 }

declare ptr @calloc(i64, i64)
declare void @free(ptr)
declare void @prion_str_free(ptr)

@.m_open = private unnamed_addr constant [2 x i8] c"{\00"
@.m_close = private unnamed_addr constant [2 x i8] c"}\00"
@.m_cint = private unnamed_addr constant [7 x i8] c": %lld\00"
@.m_cg = private unnamed_addr constant [5 x i8] c": %g\00"
@.m_cstr = private unnamed_addr constant [7 x i8] c": \22%s\22\00"

define internal ptr @prion_map_kslot(ptr %m, i64 %i) {
entry:
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 6
  %k = load ptr, ptr %fk
  %fz = getelementptr %rt.M, ptr %m, i32 0, i32 5
  %ksz = load i64, ptr %fz
  %o = mul i64 %i, %ksz
  %p = getelementptr i8, ptr %k, i64 %o
  ret ptr %p
}

define internal void @prion_map_kdesc(ptr %sret, ptr %m, i64 %i) {
entry:
  %p = call ptr @prion_map_kslot(ptr %m, i64 %i)
  %v = load %rt.S, ptr %p
  store %rt.S %v, ptr %sret
  ret void
}

define internal void @prion_map_kput(ptr %m, i64 %i, ptr %s) {
entry:
  %p = call ptr @prion_map_kslot(ptr %m, i64 %i)
  %isn = icmp eq ptr %s, null
  br i1 %isn, label %emp, label %cp
emp:
  store %rt.S zeroinitializer, ptr %p
  ret void
cp:
  %tmp = alloca %rt.S
  call void @prion_str_copy(ptr %tmp, ptr %s)
  %v = load %rt.S, ptr %tmp
  store %rt.S %v, ptr %p
  ret void
}

define internal i64 @prion_map_kint(ptr %m, i64 %i) {
entry:
  %p = call ptr @prion_map_kslot(ptr %m, i64 %i)
  %v = load i64, ptr %p
  ret i64 %v
}

define internal void @prion_map_kint_put(ptr %m, i64 %i, i64 %v) {
entry:
  %p = call ptr @prion_map_kslot(ptr %m, i64 %i)
  store i64 %v, ptr %p
  ret void
}

define internal i64 @prion_map_hash(i64 %k, i64 %kty) {
entry:
  %isint = icmp ne i64 %kty, 0
  br i1 %isint, label %int, label %str
str:
  %dn = icmp eq i64 %k, 0
  br i1 %dn, label %base, label %dp
dp:
  %d = inttoptr i64 %k to ptr
  %pp = getelementptr %rt.S, ptr %d, i32 0, i32 0
  %s = load ptr, ptr %pp
  %sn = icmp eq ptr %s, null
  br i1 %sn, label %base, label %spre
spre:
  %np = getelementptr %rt.S, ptr %d, i32 0, i32 1
  %n = load i64, ptr %np
  br label %loop
loop:
  %i = phi i64 [ 0, %spre ], [ %in, %body ]
  %h = phi i64 [ 1469598103934665603, %spre ], [ %h2, %body ]
  %lt = icmp slt i64 %i, %n
  br i1 %lt, label %body, label %retstr
body:
  %cp = getelementptr i8, ptr %s, i64 %i
  %c = load i8, ptr %cp
  %ce = zext i8 %c to i64
  %x = xor i64 %h, %ce
  %h2 = mul i64 %x, 1099511628211
  %in = add i64 %i, 1
  br label %loop
retstr:
  ret i64 %h
base:
  ret i64 1469598103934665603
int:
  %s1 = lshr i64 %k, 33
  %x1 = xor i64 %k, %s1
  %m1 = mul i64 %x1, 18397679294719823053
  %s2 = lshr i64 %m1, 33
  %x2 = xor i64 %m1, %s2
  %m2 = mul i64 %x2, 14181476777654086739
  %s3 = lshr i64 %m2, 33
  %x3 = xor i64 %m2, %s3
  ret i64 %x3
}

define internal i64 @prion_map_keyeq(i64 %slot, i64 %b, i64 %kty) {
entry:
  %isint = icmp ne i64 %kty, 0
  br i1 %isint, label %int, label %str
int:
  %ap = inttoptr i64 %slot to ptr
  %a = load i64, ptr %ap
  %eq = icmp eq i64 %a, %b
  %r = zext i1 %eq to i64
  ret i64 %r
str:
  %xp = inttoptr i64 %slot to ptr
  %xpp = getelementptr %rt.S, ptr %xp, i32 0, i32 0
  %xv = load ptr, ptr %xpp
  %bn = icmp eq i64 %b, 0
  br i1 %bn, label %chk, label %ychk
ychk:
  %bp = inttoptr i64 %b to ptr
  %ypp = getelementptr %rt.S, ptr %bp, i32 0, i32 0
  %yv = load ptr, ptr %ypp
  br label %chk
chk:
  %yvv = phi ptr [ null, %str ], [ %yv, %ychk ]
  %bpp = phi ptr [ null, %str ], [ %bp, %ychk ]
  %xnull = icmp eq ptr %xv, null
  %ynull = icmp eq ptr %yvv, null
  %any = or i1 %xnull, %ynull
  br i1 %any, label %bothnull, label %cmp
bothnull:
  %both = and i1 %xnull, %ynull
  %r2 = zext i1 %both to i64
  ret i64 %r2
cmp:
  %xnp = getelementptr %rt.S, ptr %xp, i32 0, i32 1
  %xn = load i64, ptr %xnp
  %ynp = getelementptr %rt.S, ptr %bpp, i32 0, i32 1
  %yn = load i64, ptr %ynp
  %eqn = icmp eq i64 %xn, %yn
  br i1 %eqn, label %do, label %zero
do:
  %c = call i32 @memcmp(ptr %xv, ptr %yvv, i64 %xn)
  %z = icmp eq i32 %c, 0
  %r3 = zext i1 %z to i64
  ret i64 %r3
zero:
  ret i64 0
}

define internal void @prion_map_rebuild(ptr %m) {
entry:
  %fbk = getelementptr %rt.M, ptr %m, i32 0, i32 8
  %bk = load ptr, ptr %fbk
  %fbc = getelementptr %rt.M, ptr %m, i32 0, i32 9
  %bcap = load i64, ptr %fbc
  br label %zero
zero:
  %zi = phi i64 [ 0, %entry ], [ %zin, %zbody ]
  %zlt = icmp slt i64 %zi, %bcap
  br i1 %zlt, label %zbody, label %init
zbody:
  %zp = getelementptr i64, ptr %bk, i64 %zi
  store i64 0, ptr %zp
  %zin = add i64 %zi, 1
  br label %zero
init:
  br label %loop
loop:
  %i = phi i64 [ 0, %init ], [ %in, %lnext ]
  %f0 = getelementptr %rt.M, ptr %m, i32 0, i32 0
  %len = load i64, ptr %f0
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %fkty = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fkty
  %isint = icmp ne i64 %kty, 0
  br i1 %isint, label %kint, label %kstr
kint:
  %ki = call i64 @prion_map_kint(ptr %m, i64 %i)
  br label %hash
kstr:
  %ksp = call ptr @prion_map_kslot(ptr %m, i64 %i)
  %ksi = ptrtoint ptr %ksp to i64
  br label %hash
hash:
  %kv = phi i64 [ %ki, %kint ], [ %ksi, %kstr ]
  %h = call i64 @prion_map_hash(i64 %kv, i64 %kty)
  %mask = sub i64 %bcap, 1
  %b0 = and i64 %h, %mask
  br label %probe
probe:
  %b = phi i64 [ %b0, %hash ], [ %bn, %pnext ]
  %bp = getelementptr i64, ptr %bk, i64 %b
  %bv = load i64, ptr %bp
  %nz = icmp ne i64 %bv, 0
  br i1 %nz, label %pnext, label %pset
pnext:
  %b1 = add i64 %b, 1
  %bn = and i64 %b1, %mask
  br label %probe
pset:
  %ip = add i64 %i, 1
  store i64 %ip, ptr %bp
  br label %lnext
lnext:
  %in = add i64 %i, 1
  br label %loop
done:
  ret void
}

define internal void @prion_map_grow_buckets(ptr %m, i64 %nb) {
entry:
  %sz = mul i64 %nb, 8
  %nk = call ptr @malloc(i64 %sz)
  %nn = icmp eq ptr %nk, null
  br i1 %nn, label %oom, label %go
oom:
  call void @prion_die_nc(i32 6, ptr @.s_oom1, ptr @.s_oom2)
  ret void
go:
  %fbk = getelementptr %rt.M, ptr %m, i32 0, i32 8
  %old = load ptr, ptr %fbk
  call void @free(ptr %old)
  store ptr %nk, ptr %fbk
  %fbc = getelementptr %rt.M, ptr %m, i32 0, i32 9
  store i64 %nb, ptr %fbc
  call void @prion_map_rebuild(ptr %m)
  ret void
}

define ptr @prion_map_new(i64 %esz, i64 %sgn, i64 %kty) {
entry:
  %m = call ptr @malloc(i64 80)
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %oom, label %init
init:
  %f0 = getelementptr %rt.M, ptr %m, i32 0, i32 0
  store i64 0, ptr %f0
  %f1 = getelementptr %rt.M, ptr %m, i32 0, i32 1
  store i64 4, ptr %f1
  %f2 = getelementptr %rt.M, ptr %m, i32 0, i32 2
  store i64 %esz, ptr %f2
  %f3 = getelementptr %rt.M, ptr %m, i32 0, i32 3
  store i64 %sgn, ptr %f3
  %f4 = getelementptr %rt.M, ptr %m, i32 0, i32 4
  store i64 %kty, ptr %f4
  %isz = icmp eq i64 %kty, 0
  %ksz = select i1 %isz, i64 16, i64 8
  %f5 = getelementptr %rt.M, ptr %m, i32 0, i32 5
  store i64 %ksz, ptr %f5
  %ks = mul i64 %ksz, 4
  %k = call ptr @malloc(i64 %ks)
  %f6 = getelementptr %rt.M, ptr %m, i32 0, i32 6
  store ptr %k, ptr %f6
  %vs = mul i64 %esz, 4
  %v = call ptr @malloc(i64 %vs)
  %f7 = getelementptr %rt.M, ptr %m, i32 0, i32 7
  store ptr %v, ptr %f7
  %bk = call ptr @calloc(i64 8, i64 8)
  %f8 = getelementptr %rt.M, ptr %m, i32 0, i32 8
  store ptr %bk, ptr %f8
  %f9 = getelementptr %rt.M, ptr %m, i32 0, i32 9
  store i64 8, ptr %f9
  %kn = icmp eq ptr %k, null
  %vn = icmp eq ptr %v, null
  %bn = icmp eq ptr %bk, null
  %b0 = or i1 %kn, %vn
  %b1 = or i1 %b0, %bn
  br i1 %b1, label %oom, label %ok
oom:
  call void @prion_die_nc(i32 6, ptr @.s_oom1, ptr @.s_oom2)
  ret ptr null
ok:
  ret ptr %m
}

define i64 @prion_map_len(ptr %m) {
entry:
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %zero, label %go
zero:
  ret i64 0
go:
  %f0 = getelementptr %rt.M, ptr %m, i32 0, i32 0
  %n = load i64, ptr %f0
  ret i64 %n
}

define void @prion_map_reserve(ptr %m, i64 %n) {
entry:
  %mn = icmp eq ptr %m, null
  %le = icmp sle i64 %n, 0
  %bad = or i1 %mn, %le
  br i1 %bad, label %done, label %chk
chk:
  %fcap = getelementptr %rt.M, ptr %m, i32 0, i32 1
  %cap = load i64, ptr %fcap
  %gt = icmp sgt i64 %n, %cap
  br i1 %gt, label %grow, label %buckets
grow:
  %fz = getelementptr %rt.M, ptr %m, i32 0, i32 5
  %ksz = load i64, ptr %fz
  %ks = mul i64 %ksz, %n
  %fkp = getelementptr %rt.M, ptr %m, i32 0, i32 6
  %k0 = load ptr, ptr %fkp
  %nk = call ptr @realloc(ptr %k0, i64 %ks)
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %vs = mul i64 %esz, %n
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %v0 = load ptr, ptr %fvp
  %nv = call ptr @realloc(ptr %v0, i64 %vs)
  %nkn = icmp eq ptr %nk, null
  %nvn = icmp eq ptr %nv, null
  %bd = or i1 %nkn, %nvn
  br i1 %bd, label %oom, label %setcap
oom:
  call void @prion_die_nc(i32 6, ptr @.s_oom1, ptr @.s_oom2)
  br label %done
setcap:
  store ptr %nk, ptr %fkp
  store ptr %nv, ptr %fvp
  store i64 %n, ptr %fcap
  br label %buckets
buckets:
  %fbc = getelementptr %rt.M, ptr %m, i32 0, i32 9
  %bc = load i64, ptr %fbc
  %tw = mul i64 %n, 2
  br label %bloop
bloop:
  %nb = phi i64 [ %bc, %buckets ], [ %nb2, %bbody ]
  %lt = icmp slt i64 %nb, %tw
  br i1 %lt, label %bbody, label %bdone
bbody:
  %nb2 = mul i64 %nb, 2
  br label %bloop
bdone:
  %grown = icmp sgt i64 %nb, %bc
  br i1 %grown, label %do, label %done
do:
  call void @prion_map_grow_buckets(ptr %m, i64 %nb)
  br label %done
done:
  ret void
}

define void @prion_map_free(ptr %m, i64 %vstr) {
entry:
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %done, label %init
init:
  %f0 = getelementptr %rt.M, ptr %m, i32 0, i32 0
  %len = load i64, ptr %f0
  br label %loop
loop:
  %i = phi i64 [ 0, %init ], [ %in, %next ]
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %frees
body:
  %fkty = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fkty
  %kstr = icmp eq i64 %kty, 0
  br i1 %kstr, label %fk, label %vv
fk:
  %kd = alloca %rt.S
  call void @prion_map_kdesc(ptr %kd, ptr %m, i64 %i)
  %kp = getelementptr %rt.S, ptr %kd, i32 0, i32 0
  %kv = load ptr, ptr %kp
  call void @prion_str_free(ptr %kv)
  br label %vv
vv:
  %vn = icmp eq i64 %vstr, 0
  br i1 %vn, label %next, label %vfree
vfree:
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fs = getelementptr %rt.M, ptr %m, i32 0, i32 3
  %sgn = load i64, ptr %fs
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  %pv = call i64 @prion_slot_get(ptr %sl, i64 %esz, i64 %sgn)
  %pp = inttoptr i64 %pv to ptr
  call void @prion_str_free(ptr %pp)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
frees:
  %fkp = getelementptr %rt.M, ptr %m, i32 0, i32 6
  %kp2 = load ptr, ptr %fkp
  call void @free(ptr %kp2)
  %fvp2 = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp2 = load ptr, ptr %fvp2
  call void @free(ptr %vp2)
  %fbk = getelementptr %rt.M, ptr %m, i32 0, i32 8
  %bk = load ptr, ptr %fbk
  call void @free(ptr %bk)
  call void @free(ptr %m)
  br label %done
done:
  ret void
}

define internal i64 @prion_map_probe_n(ptr %m, i64 %k, i64 %kty) {
entry:
  %h = call i64 @prion_map_hash(i64 %k, i64 %kty)
  %fbc = getelementptr %rt.M, ptr %m, i32 0, i32 9
  %bc = load i64, ptr %fbc
  %mask = sub i64 %bc, 1
  %b0 = and i64 %h, %mask
  %fbk = getelementptr %rt.M, ptr %m, i32 0, i32 8
  %bk = load ptr, ptr %fbk
  br label %loop
loop:
  %b = phi i64 [ %b0, %entry ], [ %bn, %next ]
  %p = getelementptr i64, ptr %bk, i64 %b
  %v = load i64, ptr %p
  %nz = icmp ne i64 %v, 0
  br i1 %nz, label %body, label %miss
body:
  %ii = sub i64 %v, 1
  %ks = call ptr @prion_map_kslot(ptr %m, i64 %ii)
  %ksi = ptrtoint ptr %ks to i64
  %eq = call i64 @prion_map_keyeq(i64 %ksi, i64 %k, i64 %kty)
  %hit = icmp ne i64 %eq, 0
  br i1 %hit, label %found, label %next
found:
  ret i64 %ii
next:
  %b1 = add i64 %b, 1
  %bn = and i64 %b1, %mask
  br label %loop
miss:
  %neg = sub i64 -2, %b
  ret i64 %neg
}

define i64 @prion_map_find_n(ptr %m, i64 %k, i64 %kty) {
entry:
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %neg, label %chk
chk:
  %f0 = getelementptr %rt.M, ptr %m, i32 0, i32 0
  %len = load i64, ptr %f0
  %z = icmp eq i64 %len, 0
  br i1 %z, label %neg, label %go
go:
  %i = call i64 @prion_map_probe_n(ptr %m, i64 %k, i64 %kty)
  %ok = icmp sge i64 %i, 0
  br i1 %ok, label %pos, label %neg
pos:
  ret i64 %i
neg:
  ret i64 -1
}

define void @prion_map_die_key(ptr %m, i64 %k) {
entry:
  %b = alloca [192 x i8]
  %bp = getelementptr [192 x i8], ptr %b, i64 0, i64 0
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %intf, label %kchk
kchk:
  %fkty = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fkty
  %isstr = icmp eq i64 %kty, 0
  br i1 %isstr, label %strf, label %intf
strf:
  %kn = icmp eq i64 %k, 0
  br i1 %kn, label %stremp, label %strv
strv:
  %dp = inttoptr i64 %k to ptr
  %pp = getelementptr %rt.S, ptr %dp, i32 0, i32 0
  %s = load ptr, ptr %pp
  %sn = icmp eq ptr %s, null
  br i1 %sn, label %stremp, label %strfmt
stremp:
  %c0 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %bp, i64 192, ptr @.s_mkey, ptr @.empty)
  br label %die
strfmt:
  %c1 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %bp, i64 192, ptr @.s_mkey, ptr %s)
  br label %die
intf:
  %c2 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %bp, i64 192, ptr @.s_mkeyi, i64 %k)
  br label %die
die:
  call void @prion_die_nc(i32 5, ptr %bp, ptr @.s_mkeyfix)
  ret void
}

define void @prion_map_die_key_at(ptr %m, i64 %k, ptr %f, i64 %ln, i64 %cl) {
entry:
  %b = alloca [192 x i8]
  %bp = getelementptr [192 x i8], ptr %b, i64 0, i64 0
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %intf, label %kchk
kchk:
  %fkty = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fkty
  %isstr = icmp eq i64 %kty, 0
  br i1 %isstr, label %strf, label %intf
strf:
  %kn = icmp eq i64 %k, 0
  br i1 %kn, label %stremp, label %strv
strv:
  %dp = inttoptr i64 %k to ptr
  %pp = getelementptr %rt.S, ptr %dp, i32 0, i32 0
  %s = load ptr, ptr %pp
  %sn = icmp eq ptr %s, null
  br i1 %sn, label %stremp, label %strfmt
stremp:
  %c0 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %bp, i64 192, ptr @.s_mkey, ptr @.empty)
  br label %die
strfmt:
  %c1 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %bp, i64 192, ptr @.s_mkey, ptr %s)
  br label %die
intf:
  %c2 = call i32 (ptr, i64, ptr, ...) @snprintf(ptr %bp, i64 192, ptr @.s_mkeyi, i64 %k)
  br label %die
die:
  call void @prion_die_at(ptr %f, i64 %ln, i64 %cl, ptr null, i32 5, ptr %bp, ptr @.s_mkeyfix)
  ret void
}

define i64 @prion_map_contains_n(ptr %m, i64 %k, i64 %kty) {
entry:
  %i = call i64 @prion_map_find_n(ptr %m, i64 %k, i64 %kty)
  %ok = icmp sge i64 %i, 0
  %r = zext i1 %ok to i64
  ret i64 %r
}

define i64 @prion_map_at_n(ptr %m, i64 %k, i64 %kty, i64 %esz, i64 %sgn) {
entry:
  %i = call i64 @prion_map_find_n(ptr %m, i64 %k, i64 %kty)
  %neg = icmp slt i64 %i, 0
  %on = and i1 @@ON@@, %neg
  br i1 %on, label %die, label %go
die:
  call void @prion_map_die_key(ptr %m, i64 %k)
  br label %go
go:
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  %r = call i64 @prion_slot_get(ptr %sl, i64 %esz, i64 %sgn)
  ret i64 %r
}

define i64 @prion_map_at_n_loc(ptr %m, i64 %k, i64 %kty, i64 %esz, i64 %sgn, ptr %f, i64 %ln, i64 %cl, ptr %u) {
entry:
  %i = call i64 @prion_map_find_n(ptr %m, i64 %k, i64 %kty)
  %neg = icmp slt i64 %i, 0
  %on = and i1 @@ON@@, %neg
  br i1 %on, label %die, label %go
die:
  call void @prion_map_die_key_at(ptr %m, i64 %k, ptr %f, i64 %ln, i64 %cl)
  br label %go
go:
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  %r = call i64 @prion_slot_get(ptr %sl, i64 %esz, i64 %sgn)
  ret i64 %r
}

define i64 @prion_map_at_loc(ptr %m, i64 %k, ptr %f, i64 %ln, i64 %cl, ptr %u) {
entry:
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fk
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fs = getelementptr %rt.M, ptr %m, i32 0, i32 3
  %sgn = load i64, ptr %fs
  %r = call i64 @prion_map_at_n_loc(ptr %m, i64 %k, i64 %kty, i64 %esz, i64 %sgn, ptr %f, i64 %ln, i64 %cl, ptr %u)
  ret i64 %r
}

define void @prion_map_at_s(ptr %sret, ptr %m, i64 %k, i64 %kty) {
entry:
  %i = call i64 @prion_map_find_n(ptr %m, i64 %k, i64 %kty)
  %neg = icmp slt i64 %i, 0
  %on = and i1 @@ON@@, %neg
  br i1 %on, label %die, label %go
die:
  call void @prion_map_die_key(ptr %m, i64 %k)
  br label %go
go:
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  %v = load %rt.S, ptr %sl
  store %rt.S %v, ptr %sret
  ret void
}

define void @prion_map_at_s_loc(ptr %sret, ptr %m, i64 %k, i64 %kty, ptr %f, i64 %ln, i64 %cl, ptr %u) {
entry:
  %i = call i64 @prion_map_find_n(ptr %m, i64 %k, i64 %kty)
  %neg = icmp slt i64 %i, 0
  %on = and i1 @@ON@@, %neg
  br i1 %on, label %die, label %go
die:
  call void @prion_map_die_key_at(ptr %m, i64 %k, ptr %f, i64 %ln, i64 %cl)
  br label %go
go:
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  %v = load %rt.S, ptr %sl
  store %rt.S %v, ptr %sret
  ret void
}

define void @prion_map_set_s(ptr %m, i64 %k, ptr %v, i64 %kty) {
entry:
  %i = call i64 @prion_map_find_n(ptr %m, i64 %k, i64 %kty)
  %neg = icmp slt i64 %i, 0
  %on = and i1 @@ON@@, %neg
  br i1 %on, label %die, label %go
die:
  call void @prion_map_die_key(ptr %m, i64 %k)
  br label %go
go:
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  %tmp = alloca %rt.S
  call void @prion_str_copy(ptr %tmp, ptr %v)
  %sv = load %rt.S, ptr %tmp
  store %rt.S %sv, ptr %sl
  ret void
}

define void @prion_map_put_s(ptr %m, i64 %k, ptr %v, i64 %kty) {
entry:
  %i = call i64 @prion_map_probe_n(ptr %m, i64 %k, i64 %kty)
  %found = icmp sge i64 %i, 0
  br i1 %found, label %upd, label %ins
upd:
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  %tmp = alloca %rt.S
  call void @prion_str_copy(ptr %tmp, ptr %v)
  %sv = load %rt.S, ptr %tmp
  store %rt.S %sv, ptr %sl
  ret void
ins:
  %b = sub i64 -2, %i
  %f0 = getelementptr %rt.M, ptr %m, i32 0, i32 0
  %len = load i64, ptr %f0
  %fcap = getelementptr %rt.M, ptr %m, i32 0, i32 1
  %cap = load i64, ptr %fcap
  %full = icmp sge i64 %len, %cap
  br i1 %full, label %grow, label %store
grow:
  %ncap = mul i64 %cap, 2
  %fz = getelementptr %rt.M, ptr %m, i32 0, i32 5
  %ksz = load i64, ptr %fz
  %ks = mul i64 %ksz, %ncap
  %fkp = getelementptr %rt.M, ptr %m, i32 0, i32 6
  %k0 = load ptr, ptr %fkp
  %nk = call ptr @realloc(ptr %k0, i64 %ks)
  %fe2 = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz2 = load i64, ptr %fe2
  %vs = mul i64 %esz2, %ncap
  %fvp2 = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %v0 = load ptr, ptr %fvp2
  %nv = call ptr @realloc(ptr %v0, i64 %vs)
  %nkn = icmp eq ptr %nk, null
  %nvn = icmp eq ptr %nv, null
  %bd = or i1 %nkn, %nvn
  br i1 %bd, label %oom, label %setcap
oom:
  call void @prion_die_nc(i32 6, ptr @.s_oom1, ptr @.s_oom2)
  br label %store
setcap:
  store ptr %nk, ptr %fkp
  store ptr %nv, ptr %fvp2
  store i64 %ncap, ptr %fcap
  br label %store
store:
  %isz = icmp eq i64 %kty, 0
  br i1 %isz, label %kstr, label %kint
kstr:
  %ki = inttoptr i64 %k to ptr
  call void @prion_map_kput(ptr %m, i64 %len, ptr %ki)
  br label %vput
kint:
  call void @prion_map_kint_put(ptr %m, i64 %len, i64 %k)
  br label %vput
vput:
  %fvp3 = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp3 = load ptr, ptr %fvp3
  %fe3 = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz3 = load i64, ptr %fe3
  %o3 = mul i64 %len, %esz3
  %sl3 = getelementptr i8, ptr %vp3, i64 %o3
  %tmp3 = alloca %rt.S
  call void @prion_str_copy(ptr %tmp3, ptr %v)
  %sv3 = load %rt.S, ptr %tmp3
  store %rt.S %sv3, ptr %sl3
  %nl = add i64 %len, 1
  store i64 %nl, ptr %f0
  %tw = mul i64 %nl, 2
  %fbc = getelementptr %rt.M, ptr %m, i32 0, i32 9
  %bc = load i64, ptr %fbc
  %need = icmp sgt i64 %tw, %bc
  br i1 %need, label %growb, label %setb
growb:
  %bc2 = mul i64 %bc, 2
  call void @prion_map_grow_buckets(ptr %m, i64 %bc2)
  ret void
setb:
  %fbk = getelementptr %rt.M, ptr %m, i32 0, i32 8
  %bk = load ptr, ptr %fbk
  %bp = getelementptr i64, ptr %bk, i64 %b
  store i64 %nl, ptr %bp
  ret void
}

define void @prion_map_set_n(ptr %m, i64 %k, i64 %v, i64 %kty, i64 %esz) {
entry:
  %i = call i64 @prion_map_find_n(ptr %m, i64 %k, i64 %kty)
  %neg = icmp slt i64 %i, 0
  %on = and i1 @@ON@@, %neg
  br i1 %on, label %die, label %go
die:
  call void @prion_map_die_key(ptr %m, i64 %k)
  br label %go
go:
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  call void @prion_slot_put(ptr %sl, i64 %esz, i64 %v)
  ret void
}

define void @prion_map_put_n(ptr %m, i64 %k, i64 %v, i64 %kty, i64 %esz) {
entry:
  %i = call i64 @prion_map_probe_n(ptr %m, i64 %k, i64 %kty)
  %found = icmp sge i64 %i, 0
  br i1 %found, label %upd, label %ins
upd:
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  call void @prion_slot_put(ptr %sl, i64 %esz, i64 %v)
  ret void
ins:
  %b = sub i64 -2, %i
  %f0 = getelementptr %rt.M, ptr %m, i32 0, i32 0
  %len = load i64, ptr %f0
  %fcap = getelementptr %rt.M, ptr %m, i32 0, i32 1
  %cap = load i64, ptr %fcap
  %full = icmp sge i64 %len, %cap
  br i1 %full, label %grow, label %store
grow:
  %ncap = mul i64 %cap, 2
  %fz = getelementptr %rt.M, ptr %m, i32 0, i32 5
  %ksz = load i64, ptr %fz
  %ks = mul i64 %ksz, %ncap
  %fkp = getelementptr %rt.M, ptr %m, i32 0, i32 6
  %k0 = load ptr, ptr %fkp
  %nk = call ptr @realloc(ptr %k0, i64 %ks)
  %fe2 = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz2 = load i64, ptr %fe2
  %vs = mul i64 %esz2, %ncap
  %fvp2 = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %v0 = load ptr, ptr %fvp2
  %nv = call ptr @realloc(ptr %v0, i64 %vs)
  %nkn = icmp eq ptr %nk, null
  %nvn = icmp eq ptr %nv, null
  %bd = or i1 %nkn, %nvn
  br i1 %bd, label %oom, label %setcap
oom:
  call void @prion_die_nc(i32 6, ptr @.s_oom1, ptr @.s_oom2)
  br label %store
setcap:
  store ptr %nk, ptr %fkp
  store ptr %nv, ptr %fvp2
  store i64 %ncap, ptr %fcap
  br label %store
store:
  %isz = icmp eq i64 %kty, 0
  br i1 %isz, label %kstr, label %kint
kstr:
  %ki = inttoptr i64 %k to ptr
  call void @prion_map_kput(ptr %m, i64 %len, ptr %ki)
  br label %vput
kint:
  call void @prion_map_kint_put(ptr %m, i64 %len, i64 %k)
  br label %vput
vput:
  %fvp3 = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp3 = load ptr, ptr %fvp3
  %o3 = mul i64 %len, %esz
  %sl3 = getelementptr i8, ptr %vp3, i64 %o3
  call void @prion_slot_put(ptr %sl3, i64 %esz, i64 %v)
  %nl = add i64 %len, 1
  store i64 %nl, ptr %f0
  %tw = mul i64 %nl, 2
  %fbc = getelementptr %rt.M, ptr %m, i32 0, i32 9
  %bc = load i64, ptr %fbc
  %need = icmp sgt i64 %tw, %bc
  br i1 %need, label %growb, label %setb
growb:
  %bc2 = mul i64 %bc, 2
  call void @prion_map_grow_buckets(ptr %m, i64 %bc2)
  ret void
setb:
  %fbk = getelementptr %rt.M, ptr %m, i32 0, i32 8
  %bk = load ptr, ptr %fbk
  %bp = getelementptr i64, ptr %bk, i64 %b
  store i64 %nl, ptr %bp
  ret void
}

define i64 @prion_map_find(ptr %m, i64 %k) {
entry:
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %one, label %get
one:
  br label %join
get:
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %ktyv = load i64, ptr %fk
  br label %join
join:
  %kty = phi i64 [ 1, %one ], [ %ktyv, %get ]
  %r = call i64 @prion_map_find_n(ptr %m, i64 %k, i64 %kty)
  ret i64 %r
}

define i64 @prion_map_contains(ptr %m, i64 %k) {
entry:
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %one, label %get
one:
  br label %join
get:
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %ktyv = load i64, ptr %fk
  br label %join
join:
  %kty = phi i64 [ 1, %one ], [ %ktyv, %get ]
  %r = call i64 @prion_map_contains_n(ptr %m, i64 %k, i64 %kty)
  ret i64 %r
}

define i64 @prion_map_at(ptr %m, i64 %k) {
entry:
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fk
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fs = getelementptr %rt.M, ptr %m, i32 0, i32 3
  %sgn = load i64, ptr %fs
  %r = call i64 @prion_map_at_n(ptr %m, i64 %k, i64 %kty, i64 %esz, i64 %sgn)
  ret i64 %r
}

define void @prion_map_set(ptr %m, i64 %k, i64 %v) {
entry:
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fk
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  call void @prion_map_set_n(ptr %m, i64 %k, i64 %v, i64 %kty, i64 %esz)
  ret void
}

define void @prion_map_put(ptr %m, i64 %k, i64 %v) {
entry:
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fk
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  call void @prion_map_put_n(ptr %m, i64 %k, i64 %v, i64 %kty, i64 %esz)
  ret void
}

define i64 @prion_map_remove(ptr %m, i64 %k) {
entry:
  %i = call i64 @prion_map_find(ptr %m, i64 %k)
  %neg = icmp slt i64 %i, 0
  %on = and i1 @@ON@@, %neg
  br i1 %on, label %die, label %go
die:
  call void @prion_map_die_key(ptr %m, i64 %k)
  br label %go
go:
  %f0 = getelementptr %rt.M, ptr %m, i32 0, i32 0
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fs = getelementptr %rt.M, ptr %m, i32 0, i32 3
  %sgn = load i64, ptr %fs
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  %rv = call i64 @prion_slot_get(ptr %sl, i64 %esz, i64 %sgn)
  br label %loop
loop:
  %j = phi i64 [ %i, %go ], [ %jn, %body ]
  %len = load i64, ptr %f0
  %lm = sub i64 %len, 1
  %lt = icmp slt i64 %j, %lm
  br i1 %lt, label %body, label %shrink
body:
  %k1 = call ptr @prion_map_kslot(ptr %m, i64 %j)
  %jp = add i64 %j, 1
  %k2 = call ptr @prion_map_kslot(ptr %m, i64 %jp)
  %fz = getelementptr %rt.M, ptr %m, i32 0, i32 5
  %ksz = load i64, ptr %fz
  call void @memcpy(ptr %k1, ptr %k2, i64 %ksz)
  %ov = mul i64 %j, %esz
  %v1 = getelementptr i8, ptr %vp, i64 %ov
  %ov2 = mul i64 %jp, %esz
  %v2 = getelementptr i8, ptr %vp, i64 %ov2
  call void @memcpy(ptr %v1, ptr %v2, i64 %esz)
  %jn = add i64 %j, 1
  br label %loop
shrink:
  %nl = sub i64 %len, 1
  store i64 %nl, ptr %f0
  call void @prion_map_rebuild(ptr %m)
  ret i64 %rv
}

define void @prion_map_clear(ptr %m) {
entry:
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %done, label %go
go:
  %f0 = getelementptr %rt.M, ptr %m, i32 0, i32 0
  store i64 0, ptr %f0
  %fbk = getelementptr %rt.M, ptr %m, i32 0, i32 8
  %bk = load ptr, ptr %fbk
  %fbc = getelementptr %rt.M, ptr %m, i32 0, i32 9
  %bc = load i64, ptr %fbc
  br label %loop
loop:
  %i = phi i64 [ 0, %go ], [ %in, %body ]
  %lt = icmp slt i64 %i, %bc
  br i1 %lt, label %body, label %done
body:
  %p = getelementptr i64, ptr %bk, i64 %i
  store i64 0, ptr %p
  %in = add i64 %i, 1
  br label %loop
done:
  ret void
}

define ptr @prion_map_lit(i64 %n, ptr %kp, ptr %vp, i64 %esz, i64 %sgn, i64 %kty) {
entry:
  %m = call ptr @prion_map_new(i64 %esz, i64 %sgn, i64 %kty)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %next ]
  %lt = icmp slt i64 %i, %n
  br i1 %lt, label %body, label %done
body:
  %isz = icmp eq i64 %kty, 0
  br i1 %isz, label %ks, label %ki
ks:
  %o = mul i64 %i, 16
  %p = getelementptr i8, ptr %kp, i64 %o
  %kk = ptrtoint ptr %p to i64
  %ov = mul i64 %i, 8
  %vpp = getelementptr i8, ptr %vp, i64 %ov
  %vv = load i64, ptr %vpp
  call void @prion_map_put_n(ptr %m, i64 %kk, i64 %vv, i64 %kty, i64 %esz)
  br label %next
ki:
  %o2 = mul i64 %i, 8
  %p2 = getelementptr i8, ptr %kp, i64 %o2
  %kk2 = load i64, ptr %p2
  %ov2 = mul i64 %i, 8
  %vp2 = getelementptr i8, ptr %vp, i64 %ov2
  %vv2 = load i64, ptr %vp2
  call void @prion_map_put(ptr %m, i64 %kk2, i64 %vv2)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  ret ptr %m
}

define ptr @prion_map_keys(ptr %m) {
entry:
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %nom, label %has
nom:
  br label %join
has:
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %ktyv = load i64, ptr %fk
  br label %join
join:
  %kty = phi i64 [ 0, %nom ], [ %ktyv, %has ]
  %sgnv = phi i64 [ 1, %nom ], [ %ktyv, %has ]
  %isz = icmp eq i64 %kty, 0
  %eszv = select i1 %isz, i64 16, i64 8
  %r = call ptr @prion_list_new(i64 %eszv, i64 %sgnv)
  br label %loop
loop:
  %i = phi i64 [ 0, %join ], [ %in, %next ]
  %len = call i64 @prion_map_len(ptr %m)
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %fkty = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty2 = load i64, ptr %fkty
  %isz2 = icmp eq i64 %kty2, 0
  br i1 %isz2, label %ks, label %ki
ks:
  %d = alloca %rt.S
  call void @prion_map_kdesc(ptr %d, ptr %m, i64 %i)
  %pr = call i64 @prion_list_push_s(ptr %r, ptr %d)
  br label %next
ki:
  %kv = call i64 @prion_map_kint(ptr %m, i64 %i)
  %pr2 = call i64 @prion_list_push(ptr %r, i64 %kv)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  ret ptr %r
}

define ptr @prion_map_values(ptr %m, i64 %vstr) {
entry:
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %nom, label %has
nom:
  br label %join
has:
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %eszv = load i64, ptr %fe
  %fs = getelementptr %rt.M, ptr %m, i32 0, i32 3
  %sgnv = load i64, ptr %fs
  br label %join
join:
  %esz = phi i64 [ 8, %nom ], [ %eszv, %has ]
  %sgn = phi i64 [ 1, %nom ], [ %sgnv, %has ]
  %r = call ptr @prion_list_new(i64 %esz, i64 %sgn)
  br label %loop
loop:
  %i = phi i64 [ 0, %join ], [ %in, %next ]
  %len = call i64 @prion_map_len(ptr %m)
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %sv = icmp ne i64 %vstr, 0
  br i1 %sv, label %vstrp, label %nump
vstrp:
  %fe2 = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz2 = load i64, ptr %fe2
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz2
  %sl = getelementptr i8, ptr %vp, i64 %o
  %d = alloca %rt.S
  %dv = load %rt.S, ptr %sl
  store %rt.S %dv, ptr %d
  %pr = call i64 @prion_list_push_s(ptr %r, ptr %d)
  br label %next
nump:
  %fe3 = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz3 = load i64, ptr %fe3
  %fs3 = getelementptr %rt.M, ptr %m, i32 0, i32 3
  %sgn3 = load i64, ptr %fs3
  %fvp3 = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp3 = load ptr, ptr %fvp3
  %o3 = mul i64 %i, %esz3
  %sl3 = getelementptr i8, ptr %vp3, i64 %o3
  %pv = call i64 @prion_slot_get(ptr %sl3, i64 %esz3, i64 %sgn3)
  %pr2 = call i64 @prion_list_push(ptr %r, i64 %pv)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  ret ptr %r
}

define ptr @prion_dc_map(ptr %m, i64 %str_val) {
entry:
  %mn = icmp eq ptr %m, null
  br i1 %mn, label %nul, label %go
nul:
  ret ptr null
go:
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fs = getelementptr %rt.M, ptr %m, i32 0, i32 3
  %sgn = load i64, ptr %fs
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fk
  %n = call ptr @prion_map_new(i64 %esz, i64 %sgn, i64 %kty)
  %len = call i64 @prion_map_len(ptr %m)
  br label %loop
loop:
  %i = phi i64 [ 0, %go ], [ %in, %next ]
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %sv = icmp ne i64 %str_val, 0
  br i1 %sv, label %vstr, label %kchk
vstr:
  %fe2 = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz2 = load i64, ptr %fe2
  %fvp2 = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp2 = load ptr, ptr %fvp2
  %o2 = mul i64 %i, %esz2
  %sl2 = getelementptr i8, ptr %vp2, i64 %o2
  %d = alloca %rt.S
  %dv = load %rt.S, ptr %sl2
  store %rt.S %dv, ptr %d
  %ksp = call ptr @prion_map_kslot(ptr %m, i64 %i)
  %ksi = ptrtoint ptr %ksp to i64
  %fk2 = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty2 = load i64, ptr %fk2
  call void @prion_map_put_s(ptr %n, i64 %ksi, ptr %d, i64 %kty2)
  br label %next
kchk:
  %fk3 = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty3 = load i64, ptr %fk3
  %isz = icmp eq i64 %kty3, 0
  br i1 %isz, label %kstr, label %kint
kstr:
  %kd = alloca %rt.S
  call void @prion_map_kdesc(ptr %kd, ptr %m, i64 %i)
  %kdi = ptrtoint ptr %kd to i64
  %fe4 = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %es4 = load i64, ptr %fe4
  %fs4 = getelementptr %rt.M, ptr %m, i32 0, i32 3
  %sg4 = load i64, ptr %fs4
  %fv4 = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp4 = load ptr, ptr %fv4
  %o4 = mul i64 %i, %es4
  %sl4 = getelementptr i8, ptr %vp4, i64 %o4
  %val = call i64 @prion_slot_get(ptr %sl4, i64 %es4, i64 %sg4)
  %fne = getelementptr %rt.M, ptr %n, i32 0, i32 2
  %nes = load i64, ptr %fne
  call void @prion_map_put_n(ptr %n, i64 %kdi, i64 %val, i64 0, i64 %nes)
  br label %next
kint:
  %ki = call i64 @prion_map_kint(ptr %m, i64 %i)
  %fe5 = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %es5 = load i64, ptr %fe5
  %fs5 = getelementptr %rt.M, ptr %m, i32 0, i32 3
  %sg5 = load i64, ptr %fs5
  %fv5 = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp5 = load ptr, ptr %fv5
  %o5 = mul i64 %i, %es5
  %sl5 = getelementptr i8, ptr %vp5, i64 %o5
  %val2 = call i64 @prion_slot_get(ptr %sl5, i64 %es5, i64 %sg5)
  call void @prion_map_put(ptr %n, i64 %ki, i64 %val2)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  ret ptr %n
}

define void @prion_map_print_key(i64 %k, i64 %kty) {
entry:
  %isint = icmp ne i64 %kty, 0
  br i1 %isint, label %int, label %str
int:
  %r1 = call i32 (ptr, ...) @printf(ptr @.lp_int, i64 %k)
  ret void
str:
  %dn = icmp eq i64 %k, 0
  br i1 %dn, label %emp, label %go
go:
  %dp = inttoptr i64 %k to ptr
  %pp = getelementptr %rt.S, ptr %dp, i32 0, i32 0
  %s = load ptr, ptr %pp
  %sn = icmp eq ptr %s, null
  br i1 %sn, label %emp, label %pr
emp:
  %r2 = call i32 (ptr, ...) @printf(ptr @.lp_str, ptr @.empty)
  ret void
pr:
  %r3 = call i32 (ptr, ...) @printf(ptr @.lp_str, ptr %s)
  ret void
}

define void @prion_map_print(ptr %m) {
entry:
  %r0 = call i32 (ptr, ...) @printf(ptr @.m_open)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %next ]
  %len = call i64 @prion_map_len(ptr %m)
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %gt = icmp sgt i64 %i, 0
  br i1 %gt, label %sep, label %key
sep:
  %r1 = call i32 (ptr, ...) @printf(ptr @.lp_sep)
  br label %key
key:
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fk
  %isz = icmp eq i64 %kty, 0
  br i1 %isz, label %ks, label %ki
ks:
  %p = call ptr @prion_map_kslot(ptr %m, i64 %i)
  %pi = ptrtoint ptr %p to i64
  br label %kjoin
ki:
  %iv = call i64 @prion_map_kint(ptr %m, i64 %i)
  br label %kjoin
kjoin:
  %kv = phi i64 [ %pi, %ks ], [ %iv, %ki ]
  call void @prion_map_print_key(i64 %kv, i64 %kty)
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fs = getelementptr %rt.M, ptr %m, i32 0, i32 3
  %sgn = load i64, ptr %fs
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  %val = call i64 @prion_slot_get(ptr %sl, i64 %esz, i64 %sgn)
  %r2 = call i32 (ptr, ...) @printf(ptr @.m_cint, i64 %val)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  %r3 = call i32 (ptr, ...) @printf(ptr @.m_close)
  ret void
}

define void @prion_map_print_f(ptr %m) {
entry:
  %r0 = call i32 (ptr, ...) @printf(ptr @.m_open)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %next ]
  %len = call i64 @prion_map_len(ptr %m)
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %gt = icmp sgt i64 %i, 0
  br i1 %gt, label %sep, label %key
sep:
  %r1 = call i32 (ptr, ...) @printf(ptr @.lp_sep)
  br label %key
key:
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fk
  %isz = icmp eq i64 %kty, 0
  br i1 %isz, label %ks, label %ki
ks:
  %p = call ptr @prion_map_kslot(ptr %m, i64 %i)
  %pi = ptrtoint ptr %p to i64
  br label %kjoin
ki:
  %iv = call i64 @prion_map_kint(ptr %m, i64 %i)
  br label %kjoin
kjoin:
  %kv = phi i64 [ %pi, %ks ], [ %iv, %ki ]
  call void @prion_map_print_key(i64 %kv, i64 %kty)
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fs = getelementptr %rt.M, ptr %m, i32 0, i32 3
  %sgn = load i64, ptr %fs
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  %val = call i64 @prion_slot_get(ptr %sl, i64 %esz, i64 %sgn)
  %dv = bitcast i64 %val to double
  %r2 = call i32 (ptr, ...) @printf(ptr @.m_cg, double %dv)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  %r3 = call i32 (ptr, ...) @printf(ptr @.m_close)
  ret void
}

define void @prion_map_print_s(ptr %m) {
entry:
  %r0 = call i32 (ptr, ...) @printf(ptr @.m_open)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %next ]
  %len = call i64 @prion_map_len(ptr %m)
  %lt = icmp slt i64 %i, %len
  br i1 %lt, label %body, label %done
body:
  %gt = icmp sgt i64 %i, 0
  br i1 %gt, label %sep, label %key
sep:
  %r1 = call i32 (ptr, ...) @printf(ptr @.lp_sep)
  br label %key
key:
  %fk = getelementptr %rt.M, ptr %m, i32 0, i32 4
  %kty = load i64, ptr %fk
  %isz = icmp eq i64 %kty, 0
  br i1 %isz, label %ks, label %ki
ks:
  %p = call ptr @prion_map_kslot(ptr %m, i64 %i)
  %pi = ptrtoint ptr %p to i64
  br label %kjoin
ki:
  %iv = call i64 @prion_map_kint(ptr %m, i64 %i)
  br label %kjoin
kjoin:
  %kv = phi i64 [ %pi, %ks ], [ %iv, %ki ]
  call void @prion_map_print_key(i64 %kv, i64 %kty)
  %fe = getelementptr %rt.M, ptr %m, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fvp = getelementptr %rt.M, ptr %m, i32 0, i32 7
  %vp = load ptr, ptr %fvp
  %o = mul i64 %i, %esz
  %sl = getelementptr i8, ptr %vp, i64 %o
  %sp = getelementptr %rt.S, ptr %sl, i32 0, i32 0
  %sv = load ptr, ptr %sp
  %isn = icmp eq ptr %sv, null
  %sel = select i1 %isn, ptr @.empty, ptr %sv
  %r2 = call i32 (ptr, ...) @printf(ptr @.m_cstr, ptr %sel)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  %r3 = call i32 (ptr, ...) @printf(ptr @.m_close)
  ret void
}

"##;

const ARITH_IR_BODY: &str = r##"
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64)

define i64 @prion_sadd(i64 %a, i64 %b) {
entry:
  %p = call { i64, i1 } @llvm.sadd.with.overflow.i64(i64 %a, i64 %b)
  %r = extractvalue { i64, i1 } %p, 0
  %o = extractvalue { i64, i1 } %p, 1
  %on = and i1 @@ON@@, %o
  br i1 %on, label %die, label %ok
die:
  call void @prion_die_nc(i32 3, ptr @.s_ovf, ptr @.s_ovffix)
  br label %ok
ok:
  ret i64 %r
}

define i64 @prion_ssub(i64 %a, i64 %b) {
entry:
  %p = call { i64, i1 } @llvm.ssub.with.overflow.i64(i64 %a, i64 %b)
  %r = extractvalue { i64, i1 } %p, 0
  %o = extractvalue { i64, i1 } %p, 1
  %on = and i1 @@ON@@, %o
  br i1 %on, label %die, label %ok
die:
  call void @prion_die_nc(i32 3, ptr @.s_ovf, ptr @.s_ovffix)
  br label %ok
ok:
  ret i64 %r
}

define i64 @prion_smul(i64 %a, i64 %b) {
entry:
  %p = call { i64, i1 } @llvm.smul.with.overflow.i64(i64 %a, i64 %b)
  %r = extractvalue { i64, i1 } %p, 0
  %o = extractvalue { i64, i1 } %p, 1
  %on = and i1 @@ON@@, %o
  br i1 %on, label %die, label %ok
die:
  call void @prion_die_nc(i32 3, ptr @.s_ovf, ptr @.s_ovffix)
  br label %ok
ok:
  ret i64 %r
}

define i64 @prion_div(i64 %a, i64 %b) {
entry:
  %z = icmp eq i64 %b, 0
  %on = and i1 @@ON@@, %z
  br i1 %on, label %die, label %ok
die:
  call void @prion_die_nc(i32 2, ptr @.s_div0, ptr @.s_div0fix)
  br label %ok
ok:
  %r = sdiv i64 %a, %b
  ret i64 %r
}

define i64 @prion_div_loc(i64 %a, i64 %b, ptr %f, i64 %ln, i64 %cl, ptr %u) {
entry:
  %z = icmp eq i64 %b, 0
  %on = and i1 @@ON@@, %z
  br i1 %on, label %die, label %ok
die:
  call void @prion_die_at(ptr %f, i64 %ln, i64 %cl, ptr null, i32 2, ptr @.s_div0, ptr @.s_div0fix)
  br label %ok
ok:
  %r = sdiv i64 %a, %b
  ret i64 %r
}

define i64 @prion_mod(i64 %a, i64 %b) {
entry:
  %z = icmp eq i64 %b, 0
  %on = and i1 @@ON@@, %z
  br i1 %on, label %die, label %ok
die:
  call void @prion_die_nc(i32 2, ptr @.s_div0, ptr @.s_div0fix)
  br label %ok
ok:
  %r = srem i64 %a, %b
  ret i64 %r
}

define i64 @prion_mod_loc(i64 %a, i64 %b, ptr %f, i64 %ln, i64 %cl, ptr %u) {
entry:
  %z = icmp eq i64 %b, 0
  %on = and i1 @@ON@@, %z
  br i1 %on, label %die, label %ok
die:
  call void @prion_die_at(ptr %f, i64 %ln, i64 %cl, ptr null, i32 2, ptr @.s_div0, ptr @.s_div0fix)
  br label %ok
ok:
  %r = srem i64 %a, %b
  ret i64 %r
}

define ptr @prion_copy_to_parent(ptr %src, i64 %n) {
entry:
  %d = call ptr @malloc(i64 %n)
  %sn = icmp eq ptr %src, null
  %dn = icmp eq ptr %d, null
  %bad = or i1 %sn, %dn
  br i1 %bad, label %done, label %cp
cp:
  call void @memcpy(ptr %d, ptr %src, i64 %n)
  br label %done
done:
  ret ptr %d
}
"##;

const THREAD_SHARED_IR: &str = r##"
%rt.F = type { [256 x i8], i32, i64, ptr, ptr }
%rt.T = type { ptr, i64, ptr, i64, i64, i64, %rt.F, i64, i64 }
%rt.L = type { i64, i64, i64, i64, ptr }

@prion_fail_ctx = internal thread_local global ptr null
@.t_fail = private unnamed_addr constant [13 x i8] c"\E4\BB\BB\E5\8A\A1\E5\A4\B1\E8\B4\A5\00"
@.t_empty = private unnamed_addr constant [1 x i8] zeroinitializer

declare ptr @calloc(i64, i64)
declare void @free(ptr)
declare void @memcpy(ptr, ptr, i64)
declare void @longjmp(ptr, i32)
declare void @prion_die(i32, i64, ptr, ptr)
declare i64 @prion_list_len(ptr)
declare ptr @prion_list_new(i64, i64)
declare void @prion_list_reserve(ptr, i64)
declare ptr @prion_list_slot(ptr, i64)
declare i64 @prion_slot_get(ptr, i64, i64)
declare void @prion_slot_put(ptr, i64, i64)
declare i64 @prion_list_push(ptr, i64)

define void @prion_task_fail_notify(i32 %code, i64 %line, ptr %msg, ptr %fix) {
entry:
  %c = load ptr, ptr @prion_fail_ctx
  %isn = icmp eq ptr %c, null
  br i1 %isn, label %done, label %go
go:
  %cp = getelementptr %rt.F, ptr %c, i32 0, i32 1
  store i32 %code, ptr %cp
  %lp = getelementptr %rt.F, ptr %c, i32 0, i32 2
  store i64 %line, ptr %lp
  %mp = getelementptr %rt.F, ptr %c, i32 0, i32 3
  store ptr %msg, ptr %mp
  %fp = getelementptr %rt.F, ptr %c, i32 0, i32 4
  store ptr %fix, ptr %fp
  call void @longjmp(ptr %c, i32 1)
  ret void
done:
  ret void
}

define internal void @prion_task_fail_report(ptr %c) {
entry:
  %cp = getelementptr %rt.F, ptr %c, i32 0, i32 1
  %code = load i32, ptr %cp
  %lp = getelementptr %rt.F, ptr %c, i32 0, i32 2
  %line = load i64, ptr %lp
  %mp = getelementptr %rt.F, ptr %c, i32 0, i32 3
  %msg = load ptr, ptr %mp
  %fp = getelementptr %rt.F, ptr %c, i32 0, i32 4
  %fix = load ptr, ptr %fp
  %mn = icmp eq ptr %msg, null
  %msel = select i1 %mn, ptr @.t_fail, ptr %msg
  %fn = icmp eq ptr %fix, null
  %fsel = select i1 %fn, ptr @.t_empty, ptr %fix
  call void @prion_die(i32 %code, i64 %line, ptr %msel, ptr %fsel)
  ret void
}

define internal ptr @prion_task_new(i64 %fp, ptr %ctx, i64 %n) {
entry:
  %t = call ptr @calloc(i64 1, i64 352)
  %fnp = getelementptr %rt.T, ptr %t, i32 0, i32 0
  %fpv = inttoptr i64 %fp to ptr
  store ptr %fpv, ptr %fnp
  %np = getelementptr %rt.T, ptr %t, i32 0, i32 3
  store i64 %n, ptr %np
  %gt = icmp sgt i64 %n, 0
  %cne = icmp ne ptr %ctx, null
  %docopy = and i1 %gt, %cne
  br i1 %docopy, label %copy, label %ret
copy:
  %sz = mul i64 %n, 16
  %c = call ptr @malloc(i64 %sz)
  %cp = getelementptr %rt.T, ptr %t, i32 0, i32 2
  store ptr %c, ptr %cp
  %cn2 = icmp eq ptr %c, null
  br i1 %cn2, label %ret, label %do
do:
  call void @memcpy(ptr %c, ptr %ctx, i64 %sz)
  br label %ret
ret:
  ret ptr %t
}

define ptr @prion_join_all(ptr %toks) {
entry:
  %n = call i64 @prion_list_len(ptr %toks)
  %r = call ptr @prion_list_new(i64 8, i64 1)
  %pos = icmp sgt i64 %n, 0
  %rn = select i1 %pos, i64 %n, i64 1
  call void @prion_list_reserve(ptr %r, i64 %rn)
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %in, %next ]
  %lt = icmp slt i64 %i, %n
  br i1 %lt, label %body, label %done
body:
  %fe = getelementptr %rt.L, ptr %toks, i32 0, i32 2
  %esz = load i64, ptr %fe
  %fs = getelementptr %rt.L, ptr %toks, i32 0, i32 3
  %sgn = load i64, ptr %fs
  %sp = call ptr @prion_list_slot(ptr %toks, i64 %i)
  %h = call i64 @prion_slot_get(ptr %sp, i64 %esz, i64 %sgn)
  %jv = call i64 @prion_join(i64 %h)
  %pr = call i64 @prion_list_push(ptr %r, i64 %jv)
  %sp2 = call ptr @prion_list_slot(ptr %toks, i64 %i)
  call void @prion_slot_put(ptr %sp2, i64 %esz, i64 0)
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
done:
  ret ptr %r
}
"##;

const THREAD_WIN_IR: &str = r##"
declare ptr @malloc(i64)
declare i32 @_setjmp(ptr) #0
declare ptr @CreateThread(ptr, i64, ptr, ptr, i32, ptr)
declare i32 @WaitForSingleObject(ptr, i32)
declare i32 @CloseHandle(ptr)

attributes #0 = { returns_twice }

define internal i32 @prion_task_tramp(ptr %p) {
entry:
  %fb = getelementptr %rt.T, ptr %p, i32 0, i32 6
  store ptr %fb, ptr @prion_fail_ctx
  %sr = call i32 @_setjmp(ptr %fb) #0
  %z = icmp eq i32 %sr, 0
  br i1 %z, label %body, label %fail
body:
  %fnp = getelementptr %rt.T, ptr %p, i32 0, i32 0
  %fn = load ptr, ptr %fnp
  %cp = getelementptr %rt.T, ptr %p, i32 0, i32 2
  %ctx = load ptr, ptr %cp
  %res = call i64 %fn(ptr %ctx)
  %rp = getelementptr %rt.T, ptr %p, i32 0, i32 1
  store i64 %res, ptr %rp
  br label %after
fail:
  %dp = getelementptr %rt.T, ptr %p, i32 0, i32 5
  store i64 1, ptr %dp
  br label %after
after:
  store ptr null, ptr @prion_fail_ctx
  %detp = getelementptr %rt.T, ptr %p, i32 0, i32 4
  %det = load i64, ptr %detp
  %isdet = icmp ne i64 %det, 0
  br i1 %isdet, label %detc, label %ret
detc:
  %fpp = getelementptr %rt.T, ptr %p, i32 0, i32 5
  %failed = load i64, ptr %fpp
  %hasf = icmp ne i64 %failed, 0
  br i1 %hasf, label %report, label %cleanup
report:
  call void @prion_task_fail_report(ptr %fb)
  br label %cleanup
cleanup:
  %cp2 = getelementptr %rt.T, ptr %p, i32 0, i32 2
  %ctx2 = load ptr, ptr %cp2
  call void @free(ptr %ctx2)
  call void @free(ptr %p)
  br label %ret
ret:
  ret i32 0
}

define i64 @prion_spawn(i64 %fp, ptr %ctx, i64 %n) {
entry:
  %t = call ptr @prion_task_new(i64 %fp, ptr %ctx, i64 %n)
  %h = call ptr @CreateThread(ptr null, i64 0, ptr @prion_task_tramp, ptr %t, i32 0, ptr null)
  %hb = ptrtoint ptr %h to i64
  %hp = getelementptr %rt.T, ptr %t, i32 0, i32 7
  store i64 %hb, ptr %hp
  %ti = ptrtoint ptr %t to i64
  ret i64 %ti
}

define i64 @prion_spawn_detached(i64 %fp, ptr %ctx, i64 %n) {
entry:
  %t = call ptr @prion_task_new(i64 %fp, ptr %ctx, i64 %n)
  %dp = getelementptr %rt.T, ptr %t, i32 0, i32 4
  store i64 1, ptr %dp
  %h = call ptr @CreateThread(ptr null, i64 0, ptr @prion_task_tramp, ptr %t, i32 0, ptr null)
  %hn = icmp eq ptr %h, null
  br i1 %hn, label %ret, label %close
close:
  %c = call i32 @CloseHandle(ptr %h)
  %hp = getelementptr %rt.T, ptr %t, i32 0, i32 7
  store i64 0, ptr %hp
  br label %ret
ret:
  ret i64 0
}

define i64 @prion_join(i64 %h) {
entry:
  %tn = icmp eq i64 %h, 0
  br i1 %tn, label %zero, label %go
zero:
  ret i64 0
go:
  %t = inttoptr i64 %h to ptr
  %hp = getelementptr %rt.T, ptr %t, i32 0, i32 7
  %hb = load i64, ptr %hp
  %hn = icmp eq i64 %hb, 0
  br i1 %hn, label %after, label %wait
wait:
  %hv = inttoptr i64 %hb to ptr
  %w = call i32 @WaitForSingleObject(ptr %hv, i32 -1)
  %c = call i32 @CloseHandle(ptr %hv)
  store i64 0, ptr %hp
  br label %after
after:
  %rp = getelementptr %rt.T, ptr %t, i32 0, i32 1
  %rv = load i64, ptr %rp
  %fpp = getelementptr %rt.T, ptr %t, i32 0, i32 5
  %failed = load i64, ptr %fpp
  %fbuf = alloca [288 x i8], align 16
  %fmb = getelementptr [288 x i8], ptr %fbuf, i64 0, i64 0
  %fsp = getelementptr %rt.T, ptr %t, i32 0, i32 6
  call void @memcpy(ptr %fmb, ptr %fsp, i64 288)
  %cp = getelementptr %rt.T, ptr %t, i32 0, i32 2
  %ctx = load ptr, ptr %cp
  call void @free(ptr %ctx)
  call void @free(ptr %t)
  %hasf = icmp ne i64 %failed, 0
  br i1 %hasf, label %report, label %ret
report:
  call void @prion_task_fail_report(ptr %fmb)
  br label %ret
ret:
  ret i64 %rv
}
"##;

const THREAD_POSIX_IR: &str = r##"
declare ptr @malloc(i64)
declare i32 @setjmp(ptr) #0
declare i32 @pthread_create(ptr, ptr, ptr, ptr)
declare i32 @pthread_join(i64, ptr)
declare i32 @pthread_detach(i64)

attributes #0 = { returns_twice }

define internal ptr @prion_task_tramp(ptr %p) {
entry:
  %fb = getelementptr %rt.T, ptr %p, i32 0, i32 6
  store ptr %fb, ptr @prion_fail_ctx
  %sr = call i32 @setjmp(ptr %fb) #0
  %z = icmp eq i32 %sr, 0
  br i1 %z, label %body, label %fail
body:
  %fnp = getelementptr %rt.T, ptr %p, i32 0, i32 0
  %fn = load ptr, ptr %fnp
  %cp = getelementptr %rt.T, ptr %p, i32 0, i32 2
  %ctx = load ptr, ptr %cp
  %res = call i64 %fn(ptr %ctx)
  %rp = getelementptr %rt.T, ptr %p, i32 0, i32 1
  store i64 %res, ptr %rp
  br label %after
fail:
  %dp = getelementptr %rt.T, ptr %p, i32 0, i32 5
  store i64 1, ptr %dp
  br label %after
after:
  store ptr null, ptr @prion_fail_ctx
  %detp = getelementptr %rt.T, ptr %p, i32 0, i32 4
  %det = load i64, ptr %detp
  %isdet = icmp ne i64 %det, 0
  br i1 %isdet, label %detc, label %ret
detc:
  %fpp = getelementptr %rt.T, ptr %p, i32 0, i32 5
  %failed = load i64, ptr %fpp
  %hasf = icmp ne i64 %failed, 0
  br i1 %hasf, label %report, label %cleanup
report:
  call void @prion_task_fail_report(ptr %fb)
  br label %cleanup
cleanup:
  %cp2 = getelementptr %rt.T, ptr %p, i32 0, i32 2
  %ctx2 = load ptr, ptr %cp2
  call void @free(ptr %ctx2)
  call void @free(ptr %p)
  br label %ret
ret:
  ret ptr null
}

define i64 @prion_spawn(i64 %fp, ptr %ctx, i64 %n) {
entry:
  %t = call ptr @prion_task_new(i64 %fp, ptr %ctx, i64 %n)
  %hp = getelementptr %rt.T, ptr %t, i32 0, i32 7
  %rc = call i32 @pthread_create(ptr %hp, ptr null, ptr @prion_task_tramp, ptr %t)
  %ok = icmp eq i32 %rc, 0
  %okb = zext i1 %ok to i64
  %sp = getelementptr %rt.T, ptr %t, i32 0, i32 8
  store i64 %okb, ptr %sp
  %ti = ptrtoint ptr %t to i64
  ret i64 %ti
}

define i64 @prion_spawn_detached(i64 %fp, ptr %ctx, i64 %n) {
entry:
  %t = call ptr @prion_task_new(i64 %fp, ptr %ctx, i64 %n)
  %dp = getelementptr %rt.T, ptr %t, i32 0, i32 4
  store i64 1, ptr %dp
  %hp = getelementptr %rt.T, ptr %t, i32 0, i32 7
  %rc = call i32 @pthread_create(ptr %hp, ptr null, ptr @prion_task_tramp, ptr %t)
  %ok = icmp eq i32 %rc, 0
  br i1 %ok, label %det, label %fail
det:
  %hb = load i64, ptr %hp
  %d = call i32 @pthread_detach(i64 %hb)
  br label %ret
fail:
  %cp = getelementptr %rt.T, ptr %t, i32 0, i32 2
  %ctx2 = load ptr, ptr %cp
  call void @free(ptr %ctx2)
  call void @free(ptr %t)
  br label %ret
ret:
  ret i64 0
}

define i64 @prion_join(i64 %h) {
entry:
  %tn = icmp eq i64 %h, 0
  br i1 %tn, label %zero, label %go
zero:
  ret i64 0
go:
  %t = inttoptr i64 %h to ptr
  %sp = getelementptr %rt.T, ptr %t, i32 0, i32 8
  %st = load i64, ptr %sp
  %started = icmp ne i64 %st, 0
  br i1 %started, label %dojoin, label %after
dojoin:
  %hp = getelementptr %rt.T, ptr %t, i32 0, i32 7
  %hb = load i64, ptr %hp
  %j = call i32 @pthread_join(i64 %hb, ptr null)
  store i64 0, ptr %sp
  br label %after
after:
  %rp = getelementptr %rt.T, ptr %t, i32 0, i32 1
  %rv = load i64, ptr %rp
  %fpp = getelementptr %rt.T, ptr %t, i32 0, i32 5
  %failed = load i64, ptr %fpp
  %fbuf = alloca [288 x i8], align 16
  %fmb = getelementptr [288 x i8], ptr %fbuf, i64 0, i64 0
  %fsp = getelementptr %rt.T, ptr %t, i32 0, i32 6
  call void @memcpy(ptr %fmb, ptr %fsp, i64 288)
  %cp = getelementptr %rt.T, ptr %t, i32 0, i32 2
  %ctx = load ptr, ptr %cp
  call void @free(ptr %ctx)
  call void @free(ptr %t)
  %hasf = icmp ne i64 %failed, 0
  br i1 %hasf, label %report, label %ret
report:
  call void @prion_task_fail_report(ptr %fmb)
  br label %ret
ret:
  ret i64 %rv
}
"##;

const POSIX_RUN_IR: &str = r##"
%rt.S = type { ptr, i64 }
%rt.L = type { i64, i64, i64, i64, ptr }

declare ptr @malloc(i64)
declare void @free(ptr)
declare i32 @pipe(ptr)
declare i32 @fcntl(i32, i32, ...)
declare i32 @fork()
declare i32 @close(i32)
declare i32 @open(ptr, i32, ...)
declare i32 @dup2(i32, i32)
declare i32 @execvp(ptr, ptr)
declare void @_exit(i32)
declare i64 @write(i32, ptr, i64)
declare i64 @read(i32, ptr, i64)
declare i32 @waitpid(i32, ptr, i32)
declare ptr @__errno_location()
declare i64 @prion_list_len(ptr)
declare ptr @prion_list_slot(ptr, i64)

@.p_empty = private unnamed_addr constant [1 x i8] zeroinitializer

define i64 @prion_run(ptr %prog_in, ptr %args, ptr %out_in, ptr %err_in) {
entry:
  %pnn = icmp eq ptr %prog_in, null
  br i1 %pnn, label %pg_null, label %pg_get
pg_null:
  br label %prog_join
pg_get:
  %gp = getelementptr %rt.S, ptr %prog_in, i32 0, i32 0
  %gv = load ptr, ptr %gp
  br label %prog_join
prog_join:
  %prog = phi ptr [ @.p_empty, %pg_null ], [ %gv, %pg_get ]
  %onn = icmp eq ptr %out_in, null
  br i1 %onn, label %og_null, label %og_get
og_null:
  br label %out_join
og_get:
  %op = getelementptr %rt.S, ptr %out_in, i32 0, i32 0
  %ov = load ptr, ptr %op
  br label %out_join
out_join:
  %outp = phi ptr [ null, %og_null ], [ %ov, %og_get ]
  %enn = icmp eq ptr %err_in, null
  br i1 %enn, label %eg_null, label %eg_get
eg_null:
  br label %err_join
eg_get:
  %erp = getelementptr %rt.S, ptr %err_in, i32 0, i32 0
  %erv = load ptr, ptr %erp
  br label %err_join
err_join:
  %errp = phi ptr [ null, %eg_null ], [ %erv, %eg_get ]
  %c0 = load i8, ptr %prog
  %empty = icmp eq i8 %c0, 0
  br i1 %empty, label %r404, label %go
r404:
  ret i64 -404
go:
  %n = call i64 @prion_list_len(ptr %args)
  %sz = add i64 %n, 2
  %bs = mul i64 %sz, 8
  %av = call ptr @malloc(i64 %bs)
  %avn = icmp eq ptr %av, null
  br i1 %avn, label %r10086, label %init
r10086:
  ret i64 -10086
init:
  store ptr %prog, ptr %av
  br label %loop
loop:
  %i = phi i64 [ 0, %init ], [ %in, %next ]
  %lt = icmp slt i64 %i, %n
  br i1 %lt, label %body, label %tail
body:
  %spp = call ptr @prion_list_slot(ptr %args, i64 %i)
  %dpp = getelementptr %rt.S, ptr %spp, i32 0, i32 0
  %dp = load ptr, ptr %dpp
  %dpn = icmp eq ptr %dp, null
  %sel = select i1 %dpn, ptr @.p_empty, ptr %dp
  %j = add i64 %i, 1
  %ap = getelementptr ptr, ptr %av, i64 %j
  store ptr %sel, ptr %ap
  br label %next
next:
  %in = add i64 %i, 1
  br label %loop
tail:
  %j2 = add i64 %n, 1
  %ap2 = getelementptr ptr, ptr %av, i64 %j2
  store ptr null, ptr %ap2
  %epb = alloca [2 x i32]
  %e0 = getelementptr [2 x i32], ptr %epb, i64 0, i64 0
  %r1p = getelementptr [2 x i32], ptr %epb, i64 0, i64 1
  %prc = call i32 @pipe(ptr %e0)
  %prbad = icmp ne i32 %prc, 0
  br i1 %prbad, label %pipefail, label %fcn
pipefail:
  call void @free(ptr %av)
  ret i64 -10086
fcn:
  %r0 = load i32, ptr %e0
  %r1 = load i32, ptr %r1p
  %f1 = call i32 (i32, i32, ...) @fcntl(i32 %r0, i32 2, i32 1)
  %f2 = call i32 (i32, i32, ...) @fcntl(i32 %r1, i32 2, i32 1)
  %pid = call i32 @fork()
  %neg = icmp slt i32 %pid, 0
  br i1 %neg, label %forkfail, label %forked
forkfail:
  call i32 @close(i32 %r0)
  call i32 @close(i32 %r1)
  call void @free(ptr %av)
  ret i64 -10086
forked:
  %ischild = icmp eq i32 %pid, 0
  br i1 %ischild, label %child, label %parent
child:
  call i32 @close(i32 %r0)
  %on = icmp eq ptr %outp, null
  br i1 %on, label %errchk, label %ochk
ochk:
  %oc0 = load i8, ptr %outp
  %oe = icmp eq i8 %oc0, 0
  br i1 %oe, label %errchk, label %oredir
oredir:
  %fd = call i32 (ptr, i32, ...) @open(ptr %outp, i32 577, i32 420)
  %fdneg = icmp slt i32 %fd, 0
  br i1 %fdneg, label %oerr, label %odup
oerr:
  %oep = call ptr @__errno_location()
  %ow = call i64 @write(i32 %r1, ptr %oep, i64 4)
  call void @_exit(i32 127)
  unreachable
odup:
  %d1 = call i32 @dup2(i32 %fd, i32 1)
  call i32 @close(i32 %fd)
  br label %errchk
errchk:
  %en2 = icmp eq ptr %errp, null
  br i1 %en2, label %exec, label %echk
echk:
  %ec0 = load i8, ptr %errp
  %ee = icmp eq i8 %ec0, 0
  br i1 %ee, label %exec, label %eredir
eredir:
  %fd2 = call i32 (ptr, i32, ...) @open(ptr %errp, i32 577, i32 420)
  %fd2neg = icmp slt i32 %fd2, 0
  br i1 %fd2neg, label %eerr, label %edup
eerr:
  %eep = call ptr @__errno_location()
  %ew = call i64 @write(i32 %r1, ptr %eep, i64 4)
  call void @_exit(i32 127)
  unreachable
edup:
  %d2 = call i32 @dup2(i32 %fd2, i32 2)
  call i32 @close(i32 %fd2)
  br label %exec
exec:
  %xr = call i32 @execvp(ptr %prog, ptr %av)
  %xep = call ptr @__errno_location()
  %xw = call i64 @write(i32 %r1, ptr %xep, i64 4)
  call void @_exit(i32 127)
  unreachable
parent:
  call i32 @close(i32 %r1)
  %ebuf = alloca i32
  %got = call i64 @read(i32 %r0, ptr %ebuf, i64 4)
  call i32 @close(i32 %r0)
  call void @free(ptr %av)
  %stp = alloca i32
  store i32 0, ptr %stp
  br label %wloop
wloop:
  %wr = call i32 @waitpid(i32 %pid, ptr %stp, i32 0)
  %wn = icmp slt i32 %wr, 0
  br i1 %wn, label %wchk, label %wdone
wchk:
  %wep = call ptr @__errno_location()
  %wev = load i32, ptr %wep
  %intr = icmp eq i32 %wev, 4
  br i1 %intr, label %wloop, label %wdone
wdone:
  %got4 = icmp eq i64 %got, 4
  br i1 %got4, label %errout, label %st
errout:
  %sev = load i32, ptr %ebuf
  %is404 = icmp eq i32 %sev, 2
  %rsel = select i1 %is404, i64 -404, i64 -10086
  ret i64 %rsel
st:
  %stv = load i32, ptr %stp
  %low = and i32 %stv, 127
  %exited = icmp eq i32 %low, 0
  br i1 %exited, label %exok, label %r4
exok:
  %code = lshr i32 %stv, 8
  %code8 = and i32 %code, 255
  %code64 = zext i32 %code8 to i64
  ret i64 %code64
r4:
  ret i64 -4
}
"##;
